import Hydro.Sem
import Hydro.Trace
import Hydro.Wire
import Hydro.HMap
import Mathlib.Data.Prod.Lex
import Mathlib.Order.MinMax
import Mathlib.Order.Basic
import Mathlib.Data.Finset.Sort

/-!
# Paxos · message and state types (paxos.rs:33–75, 828–890)

Self-contained definitions mirroring the Rust structs. Ballots order
lexicographically by `(num, proposer_id)` (Rust `derive(PartialOrd,
Ord)`); the order algebra is proved through the key embedding into
`Lex (ℕ × ℕ)`. The acceptor log is a slot-keyed association list with
max-ballot insertion.
-/

namespace Hydro

/-- The transcription variant: `faithful` is paxos.rs verbatim (carrying
its two falsified bugs — B1 duplicate same-ballot P1a broadcasts that
double-count P1b quorums, B2 every-tick recommit/rebase that collides
`(slot, ballot)` keys); `guarded` adds the two minimal fixes. Guarantees
are variant-guarded implications — the faithful branch claims nothing. -/
inductive PaxosVariant where
  | faithful
  | guarded
deriving DecidableEq, Repr

/-- The B1 fix: send each ballot's P1a once (dedup-last on the trigger
wire). -/
def PaxosVariant.sendOnce : PaxosVariant → Bool
  | .faithful => false
  | .guarded => true

/-- The B2 fix: recommit (and rebase slot indexing) once per ballot, on
becoming leader. -/
def PaxosVariant.recommitOnce : PaxosVariant → Bool
  | .faithful => false
  | .guarded => true

/-- The B4 fix (D71): rebase fresh payloads above the recovered view's
checkpoint too — a slot at or below the checkpoint is executed, and the
acceptors have collected it (`reduce_watermark`), so a never-led proposer
must not index a fresh payload there. -/
def PaxosVariant.rebaseAboveCheckpoint : PaxosVariant → Bool
  | .faithful => false
  | .guarded => true

/-- Rust `Ballot { num: u32, proposer_id: ClusterId<Proposer> }`. -/
structure Ballot (nP : Nat) where
  num : Nat
  proposerId : Fin nP
deriving DecidableEq, Repr, Hashable, Lean.ToJson, Lean.FromJson

namespace Ballot

variable {nP : Nat}

@[simp] theorem num_mk (n : Nat) (p : Fin nP) :
    (Ballot.mk n p).num = n := rfl

@[simp] theorem proposerId_mk (n : Nat) (p : Fin nP) :
    (Ballot.mk n p).proposerId = p := rfl

/-- Lexicographic strict order (Rust `derive(Ord)` field order). -/
def blt (a b : Ballot nP) : Bool :=
  a.num < b.num || (a.num = b.num && a.proposerId.val < b.proposerId.val)

/-- Lexicographic order. -/
def ble (a b : Ballot nP) : Bool := a.blt b || a = b

/-- The order embedding into `Lex (ℕ × ℕ)`. -/
def key (a : Ballot nP) : Lex (Nat × Nat) := toLex (a.num, a.proposerId.val)

theorem key_injective : Function.Injective (key (nP := nP)) := by
  intro a b h
  have h' : (a.num, a.proposerId.val) = (b.num, b.proposerId.val) :=
    toLex.injective h
  cases a; cases b
  simp only [Prod.mk.injEq] at h'
  exact congrArg₂ Ballot.mk h'.1 (Fin.ext h'.2)

theorem blt_iff {a b : Ballot nP} :
    a.blt b = true ↔
      a.num < b.num ∨ (a.num = b.num ∧ a.proposerId.val < b.proposerId.val) := by
  unfold blt
  simp [Bool.or_eq_true, Bool.and_eq_true, decide_eq_true_iff]

theorem blt_iff_key {a b : Ballot nP} : a.blt b = true ↔ key a < key b := by
  rw [blt_iff]
  exact Iff.symm Prod.Lex.lt_iff

theorem ble_iff {a b : Ballot nP} :
    a.ble b = true ↔ a.blt b = true ∨ a = b := by
  unfold ble
  simp [Bool.or_eq_true, decide_eq_true_iff]

theorem ble_iff_key {a b : Ballot nP} : a.ble b = true ↔ key a ≤ key b := by
  rw [ble_iff, blt_iff_key, le_iff_lt_or_eq]
  exact or_congr Iff.rfl ⟨fun h => h ▸ rfl, fun h => key_injective h⟩

theorem ble_refl (a : Ballot nP) : a.ble a = true :=
  ble_iff.mpr (Or.inr rfl)

theorem ble_trans {a b c : Ballot nP} (h₁ : a.ble b = true)
    (h₂ : b.ble c = true) : a.ble c = true :=
  ble_iff_key.mpr (le_trans (ble_iff_key.mp h₁) (ble_iff_key.mp h₂))

theorem blt_irrefl (a : Ballot nP) : a.blt a = false := by
  cases h : a.blt a
  · rfl
  · exact absurd (blt_iff_key.mp h) (lt_irrefl _)

theorem blt_trans {a b c : Ballot nP} (h₁ : a.blt b = true)
    (h₂ : b.blt c = true) : a.blt c = true :=
  blt_iff_key.mpr (lt_trans (blt_iff_key.mp h₁) (blt_iff_key.mp h₂))

/-- Totality: a failed strict comparison inverts. -/
theorem ble_of_not_blt {a b : Ballot nP} (h : a.blt b = false) :
    b.ble a = true := by
  refine ble_iff_key.mpr (not_lt.mp (fun hlt => ?_))
  rw [blt_iff_key.mpr hlt] at h
  cases h

/-- Strict-from-weak: `a ≤ b` and `a ≠ b` give `a < b`. -/
theorem blt_of_ble_ne {a b : Ballot nP} (h : a.ble b = true)
    (hne : a ≠ b) : a.blt b = true := by
  rcases ble_iff.mp h with hlt | rfl
  · exact hlt
  · exact absurd rfl hne

/-- Asymmetry. -/
theorem blt_asymm {a b : Ballot nP} (h : a.blt b = true) :
    b.blt a = false := by
  cases hc : b.blt a
  · rfl
  · exact absurd (blt_trans h hc) (by rw [blt_irrefl]; exact fun h => nomatch h)

/-- `a.max b` (Rust `Ord::max`). -/
def max (a b : Ballot nP) : Ballot nP := if a.blt b then b else a

theorem key_max (a b : Ballot nP) :
    key (a.max b) = Max.max (key a) (key b) := by
  unfold max
  cases h : a.blt b
  · rw [if_neg (fun hc => nomatch hc)]
    have h' : ¬ key a < key b := fun hlt => by
      have hb := blt_iff_key.mpr hlt
      rw [h] at hb
      exact nomatch hb
    exact (max_eq_left (not_lt.mp h')).symm
  · rw [if_pos rfl]
    exact (max_eq_right (le_of_lt (blt_iff_key.mp h))).symm

theorem max_comm (a b : Ballot nP) : a.max b = b.max a :=
  key_injective (by rw [key_max, key_max, _root_.max_comm])

/-- `min` (for the lifted linear order's `Min` field). -/
def min (a b : Ballot nP) : Ballot nP := if a.blt b then a else b

theorem key_min (a b : Ballot nP) :
    key (a.min b) = Min.min (key a) (key b) := by
  unfold min
  cases h : a.blt b
  · rw [if_neg (fun hc => nomatch hc)]
    have h' : ¬ key a < key b := fun hlt => by
      have hb := blt_iff_key.mpr hlt
      rw [h] at hb
      exact nomatch hb
    exact (min_eq_right (not_lt.mp h')).symm
  · rw [if_pos rfl]
    exact (min_eq_left (le_of_lt (blt_iff_key.mp h))).symm

instance : Max (Ballot nP) := ⟨Ballot.max⟩
instance : Min (Ballot nP) := ⟨Ballot.min⟩

/-- Rust `derive(Ord)` (lexicographic): the linear order along the
`key` embedding, with `Ballot.max`/`Ballot.min` as ITS `max`/`min` —
so the generic in-tick `.max()` (`bmax`/`maxStep`) IS paxos.rs's. -/
instance : LinearOrder (Ballot nP) :=
  LinearOrder.lift key key_injective key_max key_min

/-- The lifted strict order IS `blt`. -/
theorem lt_iff_blt {a b : Ballot nP} : a < b ↔ a.blt b = true :=
  blt_iff_key.symm

/-- Among one owner's ballots, strict order is strict `num` ascent. -/
theorem num_lt_of_lt_of_owner {a b : Ballot nP} (h : a < b)
    (hown : a.proposerId = b.proposerId) : a.num < b.num := by
  rcases blt_iff.mp (lt_iff_blt.mp h) with hlt | ⟨-, hpid⟩
  · exact hlt
  · rw [hown] at hpid
    exact absurd hpid (lt_irrefl _)

/-- Fold `max` into an optional running maximum. -/
def maxOpt : Option (Ballot nP) → Ballot nP → Ballot nP
  | none, b => b
  | some a, b => a.max b

/-- `stream.max()` over a batch, from an optional current max
(paxos.rs:281–285). Commutative — the `NoOrder` fold obligation. -/
def maxFold (cur : Option (Ballot nP)) (b : Ballot nP) :
    Option (Ballot nP) :=
  some (maxOpt cur b)

theorem maxFold_comm (s : Option (Ballot nP)) (x y : Ballot nP) :
    maxFold (maxFold s x) y = maxFold (maxFold s y) x := by
  cases s with
  | none =>
    show some (x.max y) = some (y.max x)
    rw [max_comm]
  | some a =>
    show some ((a.max x).max y) = some ((a.max y).max x)
    exact congrArg some (key_injective (by
      rw [key_max, key_max, key_max, key_max]
      exact _root_.max_right_comm _ _ _))

/-- Re-absorbing a delivered ballot does not move the max — the
idempotence obligation of folding an `AtLeastOnce` pool. -/
theorem maxFold_idem (s : Option (Ballot nP)) (b : Ballot nP) :
    maxFold (maxFold s b) b = maxFold s b := by
  cases s with
  | none =>
    show some (b.max b) = some b
    exact congrArg some (key_injective (by rw [key_max, max_self]))
  | some a =>
    show some ((a.max b).max b) = some (a.max b)
    exact congrArg some (key_injective (by
      rw [key_max, key_max]
      rw [max_assoc, max_self]))

/-- `maxFold` is the generic running-max step at the lifted order. -/
theorem maxFold_eq_maxStep :
    (Ballot.maxFold (nP := nP)) = maxStep := by
  funext cur b
  cases cur <;> rfl

/-- `none`-bottomed ballot order (the acceptor max wire's value order). -/
def obtLE : Option (Ballot nP) → Option (Ballot nP) → Prop
  | none, _ => True
  | some _, none => False
  | some a, some b => a.ble b = true

@[reducible] def obtVO : ValueOrder (Option (Ballot nP)) where
  le := obtLE
  le_refl a := by
    cases a
    · trivial
    · exact ble_refl _
  le_trans {a b c} h₁ h₂ := by
    cases a <;> cases b <;> cases c <;> first
      | trivial
      | exact False.elim h₁
      | exact False.elim h₂
      | exact ble_trans h₁ h₂

/-- Absorbing a ballot never lowers the max — the `monotonic`
obligation of the acceptor's running max. -/
theorem obtLE_maxFold (s : Option (Ballot nP)) (b : Ballot nP) :
    obtLE s (maxFold s b) := by
  cases s with
  | none => trivial
  | some a =>
    show a.ble (a.max b) = true
    refine ble_iff_key.mpr ?_
    rw [key_max]
    exact le_max_left _ _

/-- `received_max_ballot <= Some(cur)` (paxos.rs:386–390). -/
def optLe : Option (Ballot nP) → Ballot nP → Bool
  | none, _ => true
  | some a, b => a.ble b

/-- Ballot-number order (the proposer ballot wire's `Monotonic` value
order — numbers only ascend). -/
@[reducible] def numVO {nP : Nat} : ValueOrder (Ballot nP) where
  le a b := a.num ≤ b.num
  le_refl _ := Nat.le_refl _
  le_trans h₁ h₂ := Nat.le_trans h₁ h₂

/-- `ble` splits into equality or strict order. -/
theorem eq_or_blt_of_ble {a b : Ballot nP} (h : a.ble b = true) :
    a = b ∨ a.blt b = true := by
  unfold ble at h
  have h' := h
  simp only [Bool.or_eq_true] at h'
  rcases h' with h1 | h2
  · exact Or.inr h1
  · exact Or.inl (of_decide_eq_true h2)

/-- `≤` one way and `<` the other cannot coexist. -/
theorem ble_blt_asymm {a b : Ballot nP} (h₁ : a.ble b = true)
    (h₂ : b.blt a = true) : False := by
  rcases eq_or_blt_of_ble h₁ with rfl | hlt
  · rw [blt_irrefl] at h₂
    cases h₂
  · have := blt_asymm hlt
    rw [h₂] at this
    cases this

/-- Same owner + same number = same ballot. -/
theorem eq_of_num_owner {a b : Ballot nP}
    (hnum : a.num = b.num) (hown : a.proposerId = b.proposerId) :
    a = b := by
  cases a
  cases b
  simp_all

end Ballot

/-- Rust `LogValue<P> { ballot, value }` (paxos.rs:828–834). -/
structure LogValue (P : Type) (nP : Nat) where
  ballot : Ballot nP
  value : Option P
deriving DecidableEq, Repr, Lean.ToJson, Lean.FromJson

/-- The acceptor log snapshot, Rust `HashMap<usize, LogValue<P>>`
(paxos.rs:890–897): the extensional hash map (`Hydro.HMap`, D75). -/
abbrev LogMap (P : Type) (nP : Nat) := HMap Nat (LogValue P nP)

/-- `Some(&p2a.ballot) >= max_ballot` (paxos.rs:841): a P2a qualifies
for the log iff its ballot is not behind the acceptor max. -/
def p2aQualifies {nP : Nat} (mb : Option (Ballot nP))
    (b : Ballot nP) : Bool :=
  match mb with
  | none => true
  | some m => m.ble b

/-- `s.max()` over a whole unordered batch (the commutativity of
`Ballot.maxFold` pays the `NoOrder` fold obligation). -/
def Ballot.maxFoldBatch {nP : Nat} (s : Option (Ballot nP))
    (b : Multiset (Ballot nP)) : Option (Ballot nP) :=
  @Multiset.foldl _ _ Ballot.maxFold
    ⟨fun s x y => Ballot.maxFold_comm s x y⟩ s b


/-- The batch max answers a member (from an empty seed). -/
theorem Ballot.maxFoldBatch_mem_of_some {nP : Nat} {b : Ballot nP}
    {ms : Multiset (Ballot nP)}
    (h : Ballot.maxFoldBatch none ms = some b) : b ∈ ms := by
  suffices hgen : ∀ (ms : Multiset (Ballot nP))
      (s : Option (Ballot nP)) (b : Ballot nP),
      Ballot.maxFoldBatch s ms = some b → some b = s ∨ b ∈ ms by
    rcases hgen ms none b h with hl | hr
    · cases hl
    · exact hr
  intro ms
  induction ms using Multiset.induction_on with
  | empty =>
    intro s b h
    exact Or.inl h.symm
  | cons x m ih =>
    intro s b h
    unfold Ballot.maxFoldBatch at h
    rw [Multiset.foldl_cons] at h
    rcases ih (Ballot.maxFold s x) b h with hl | hr
    · cases s with
      | none =>
        have hbx : some b = some x := hl
        injection hbx with hb
        exact Or.inr (by rw [hb]; exact Multiset.mem_cons_self ..)
      | some a =>
        have hbax : b = a.max x := by
          have h2 : some b = some (a.max x) := hl
          injection h2
        unfold Ballot.max at hbax
        by_cases hc : a.blt x = true
        · rw [if_pos hc] at hbax
          exact Or.inr (by rw [hbax]; exact Multiset.mem_cons_self ..)
        · rw [if_neg hc] at hbax
          exact Or.inl (by rw [hbax])
    · exact Or.inr (Multiset.mem_cons_of_mem hr)

/-- The batch max only ascends from its seed (`obtLE` state ascent
across a whole batch). -/
theorem Ballot.maxFoldBatch_le {nP : Nat} (s : Option (Ballot nP))
    (b : Multiset (Ballot nP)) :
    Ballot.obtLE s (Ballot.maxFoldBatch s b) := by
  induction b using Multiset.induction_on generalizing s with
  | empty => exact Ballot.obtVO.le_refl s
  | cons y m ih =>
    have hstep : Ballot.maxFoldBatch s (y ::ₘ m)
        = Ballot.maxFoldBatch (Ballot.maxFold s y) m := by
      unfold Ballot.maxFoldBatch
      rw [Multiset.foldl_cons]
    rw [hstep]
    exact Ballot.obtVO.le_trans (Ballot.obtLE_maxFold s y)
      (ih (Ballot.maxFold s y))

/-- The batch max dominates every member (and a nonempty batch answers
`some`). -/
theorem Ballot.maxFoldBatch_mem_ble {nP : Nat} (s : Option (Ballot nP))
    (b : Multiset (Ballot nP)) :
    ∀ x ∈ b, ∃ r, Ballot.maxFoldBatch s b = some r
      ∧ x.ble r = true := by
  induction b using Multiset.induction_on generalizing s with
  | empty =>
    intro x hx
    cases hx
  | cons y m ih =>
    intro x hx
    have hstep : Ballot.maxFoldBatch s (y ::ₘ m)
        = Ballot.maxFoldBatch (Ballot.maxFold s y) m := by
      unfold Ballot.maxFoldBatch
      rw [Multiset.foldl_cons]
    rcases Multiset.mem_cons.mp hx with rfl | hx'
    · -- `x` is absorbed now; the state only ascends afterwards
      have hasc : Ballot.obtLE (Ballot.maxFold s x)
          (Ballot.maxFoldBatch (Ballot.maxFold s x) m) :=
        Ballot.maxFoldBatch_le _ m
      cases hres : Ballot.maxFoldBatch (Ballot.maxFold s x) m with
      | none =>
        rw [hres] at hasc
        exact hasc.elim
      | some r =>
        rw [hres] at hasc
        refine ⟨r, by rw [hstep, hres], ?_⟩
        have h1 : x.ble (Ballot.maxOpt s x) = true := by
          cases s with
          | none => exact Ballot.ble_iff_key.mpr (le_refl _)
          | some a =>
            show x.ble (a.max x) = true
            refine Ballot.ble_iff_key.mpr ?_
            rw [Ballot.key_max]
            exact le_max_right _ _
        have h2 : (Ballot.maxOpt s x).ble r = true := hasc
        exact Ballot.ble_trans h1 h2
    · obtain ⟨r, hr, hle⟩ := ih (Ballot.maxFold s y) x hx'
      exact ⟨r, by rw [hstep]; exact hr, hle⟩

/-! ## The acceptor log's merge (paxos.rs:851–872)

The `reduce_watermark` closure, its tie prerequisite, and the per-slot
reduce it induces: the slot's max-ballot entry. -/

/-- **The log-merge tie**: two entries at one slot with the same ballot
carry the same value — the `assume` paxos.rs:862's `TODO` asks for
(`commutative = manual_proof!(/** max by ballot (TODO: not if two
entries with same ballot, need assume) */)`), the content half of the
acceptor log's `requires`. -/
def LogTie {P : Type} {nP : Nat} (x y : LogValue P nP) : Prop :=
  x.ballot = y.ballot → x.value = y.value

instance {P : Type} [DecidableEq P] {nP : Nat} : DecidableRel (LogTie (P := P) (nP := nP)) :=
  fun x y => inferInstanceAs (Decidable (x.ballot = y.ballot → x.value = y.value))

/-- paxos.rs:855–860: `if entry.ballot > prev_entry.ballot { *prev_entry = entry }`. -/
def logMerge {P : Type} {nP : Nat} (prev entry : LogValue P nP) : LogValue P nP :=
  if prev.ballot.blt entry.ballot then entry else prev

theorem Ballot.blt_eq_decide {nP : Nat} (a b : Ballot nP) :
    a.blt b = decide (Ballot.key a < Ballot.key b) := by
  cases h : a.blt b
  · symm; rw [decide_eq_false_iff_not]; intro hlt; rw [Ballot.blt_iff_key.mpr hlt] at h; cases h
  · symm; rw [decide_eq_true_iff]; exact Ballot.blt_iff_key.mp h

theorem LogValue.ext' {P : Type} {nP : Nat} {x y : LogValue P nP}
    (hb : x.ballot = y.ballot) (hv : x.value = y.value) : x = y := by
  cases x; cases y; cases hb; cases hv; rfl

/-- The literal merge commutes under the tie (the `commutative`
obligation of paxos.rs:862, honest). -/
theorem logMerge_comm {P : Type} {nP : Nat} (s : Option (LogValue P nP)) (x y : LogValue P nP)
    (h : LogTie x y) :
    reduceStep logMerge (reduceStep logMerge s x) y
      = reduceStep logMerge (reduceStep logMerge s y) x := by
  rcases lt_trichotomy (Ballot.key x.ballot) (Ballot.key y.ballot) with hlt | heq | hgt
  · have h1 : x.ballot.blt y.ballot = true := Ballot.blt_iff_key.mpr hlt
    have h2 : y.ballot.blt x.ballot = false := by
      rw [Ballot.blt_eq_decide, decide_eq_false_iff_not]; exact lt_asymm hlt
    cases s with
    | none => simp [reduceStep, logMerge, h1, h2]
    | some p =>
      simp only [reduceStep, logMerge]
      by_cases hpx : p.ballot.blt x.ballot = true <;> by_cases hpy : p.ballot.blt y.ballot = true
        <;> simp_all
      exact absurd (Ballot.blt_trans hpx h1) (Bool.eq_false_iff.mp hpy)
  · have hbe : x.ballot = y.ballot := Ballot.key_injective heq
    have hxy : x = y := LogValue.ext' hbe (h hbe)
    subst hxy; rfl
  · have h1 : y.ballot.blt x.ballot = true := Ballot.blt_iff_key.mpr hgt
    have h2 : x.ballot.blt y.ballot = false := by
      rw [Ballot.blt_eq_decide, decide_eq_false_iff_not]; exact lt_asymm hgt
    cases s with
    | none => simp [reduceStep, logMerge, h1, h2]
    | some p =>
      simp only [reduceStep, logMerge]
      by_cases hpx : p.ballot.blt x.ballot = true <;> by_cases hpy : p.ballot.blt y.ballot = true
        <;> simp_all
      exact absurd (Ballot.blt_trans hpy h1) (Bool.eq_false_iff.mp hpx)

/-- The per-slot reduce (Rust's `reduce_watermark` closure folded over
one slot's entries). -/
abbrev logReduce {P : Type} [DecidableEq P] {nP : Nat} (s : Option (LogValue P nP))
    (m : Multiset (LogValue P nP)) : Option (LogValue P nP) :=
  Multiset.foldlOn LogTie (reduceStep logMerge) s logMerge_comm m

theorem logMerge_ble_left {P : Type} {nP : Nat} (p e : LogValue P nP) :
    p.ballot.ble (logMerge p e).ballot = true := by
  unfold logMerge
  split_ifs with h
  · exact Ballot.ble_iff.mpr (Or.inl h)
  · exact Ballot.ble_refl _

theorem logMerge_ble_right {P : Type} {nP : Nat} (p e : LogValue P nP) :
    e.ballot.ble (logMerge p e).ballot = true := by
  unfold logMerge
  split_ifs with h
  · exact Ballot.ble_refl _
  · exact Ballot.ble_of_not_blt (Bool.eq_false_iff.mpr h)

theorem logMerge_mem {P : Type} {nP : Nat} (p e : LogValue P nP) :
    logMerge p e = p ∨ logMerge p e = e := by
  unfold logMerge; split_ifs <;> simp

/-- **The reduce is the slot's max-ballot entry**: on tie-coherent
content, the fold from a seed answers an element (or the seed) that
dominates every element and the seed. -/
theorem logReduce_spec {P : Type} [DecidableEq P] {nP : Nat} (m : Multiset (LogValue P nP))
    (hm : PairwiseM LogTie m) :
    ∀ (s : Option (LogValue P nP)),
      (logReduce s m = none ↔ m = 0 ∧ s = none)
      ∧ ∀ v, logReduce s m = some v →
          (v ∈ m ∨ s = some v) ∧ (∀ x ∈ m, x.ballot.ble v.ballot = true)
          ∧ (∀ u, s = some u → u.ballot.ble v.ballot = true) := by
  induction m using Multiset.induction_on with
  | empty =>
    intro s
    refine ⟨by simp [logReduce, Multiset.foldlOn_zero], fun v hv => ?_⟩
    simp only [logReduce, Multiset.foldlOn_zero] at hv
    subst hv
    exact ⟨Or.inr rfl, fun x hx => absurd hx (Multiset.notMem_zero _),
      fun u hu => by cases hu; exact Ballot.ble_refl _⟩
  | cons a m ih =>
    intro s
    have hstep : logReduce s (a ::ₘ m) = logReduce (reduceStep logMerge s a) m :=
      Multiset.foldlOn_cons _ _ _ _ hm
    -- the stepped seed: `a` or `logMerge p a`, dominating `a` and `p`
    have hseed : ∃ w, reduceStep logMerge s a = some w ∧ (w = a ∨ ∃ p, s = some p ∧ w = logMerge p a)
        ∧ a.ballot.ble w.ballot = true ∧ (∀ u, s = some u → u.ballot.ble w.ballot = true) := by
      cases s with
      | none => exact ⟨a, rfl, Or.inl rfl, Ballot.ble_refl _, fun u hu => by cases hu⟩
      | some p =>
        exact ⟨logMerge p a, rfl, Or.inr ⟨p, rfl, rfl⟩, logMerge_ble_right p a,
          fun u hu => by cases hu; exact logMerge_ble_left _ _⟩
    obtain ⟨w, hw, hwmem, haw, hsw⟩ := hseed
    have ih' := ih (PairwiseM.cons hm) (some w)
    rw [hstep, hw]
    refine ⟨⟨fun hn => ?_, fun ⟨h0, _⟩ => absurd h0 (Multiset.cons_ne_zero)⟩, fun v hv => ?_⟩
    · exact absurd ((ih'.1.mp hn).2) (by simp)
    · obtain ⟨hvmem, hdom, hseeddom⟩ := ih'.2 v hv
      have hwv : w.ballot.ble v.ballot = true := hseeddom w rfl
      refine ⟨?_, fun x hx => ?_, fun u hu => Ballot.ble_trans (hsw u hu) hwv⟩
      · rcases hvmem with hvm | hvw
        · exact Or.inl (Multiset.mem_cons_of_mem hvm)
        · cases hvw
          rcases hwmem with rfl | ⟨p, rfl, hw'⟩
          · exact Or.inl (Multiset.mem_cons_self _ _)
          · rcases logMerge_mem p a with hpa | hpa
            · exact Or.inr (by rw [hw', hpa])
            · exact Or.inl (by rw [hw', hpa]; exact Multiset.mem_cons_self _ _)
      · rcases Multiset.mem_cons.mp hx with rfl | hx'
        · exact Ballot.ble_trans haw hwv
        · exact hdom x hx'

/-- A pool read answers an element of the slot's group (unconditionally:
a refused group answers nothing). -/
theorem logReduce_none_some_mem {P : Type} [DecidableEq P] {nP : Nat}
    {m : Multiset (LogValue P nP)} {v : LogValue P nP}
    (h : Multiset.foldlOn LogTie (reduceStep logMerge) none logMerge_comm m = some v) :
    v ∈ m ∧ PairwiseM LogTie m := by
  by_cases hp : PairwiseM LogTie m
  · obtain ⟨-, hs⟩ := logReduce_spec m hp none
    obtain ⟨hmem, -, -⟩ := hs v h
    rcases hmem with hmem | hmem
    · exact ⟨hmem, hp⟩
    · cases hmem
  · rw [Multiset.foldlOn_of_not_pairwise _ _ _ _ hp] at h
    cases h


/-! ## Coverage (the vocabulary of the acceptor's published log)

(D71: the canonical `logView` — a commutative "honest refinement" of the
recommit fold, with a consensus tie-break — is gone; the recommit fold
is the literal order-dependent closure `rcStep` below, run on a
`tassume_ordering` decision.) -/

/-- Coverage of a slot at (or above) a ballot. -/
def LogCovers {P : Type} [DecidableEq P] {nP : Nat} (log : LogMap P nP)
    (slot : Nat) (b : Ballot nP) : Prop :=
  ∃ e : LogValue P nP, (slot, e) ∈ log ∧ b.ble e.ballot = true

/-- The p1b reply (paxos.rs:485–496): `Ok (checkpoint, log)` or
`Err max_ballot`. -/
structure P1b (P : Type) (nP : Nat) where
  ballot : Ballot nP
  res : Except (Option (Ballot nP)) (Option Nat × LogMap P nP)
deriving DecidableEq, Repr, Lean.ToJson, Lean.FromJson

/-- Rust `P2a { sender, ballot, slot, value }` (paxos.rs:64–70). -/
structure P2a (P : Type) (nP : Nat) where
  sender : Fin nP
  ballot : Ballot nP
  slot : Nat
  value : Option P
deriving DecidableEq, Repr, Lean.ToJson, Lean.FromJson

/-- The p2b ack (paxos.rs:875–890): key `(slot, ballot)`, `Ok` iff the
ballot is the acceptor's current max. -/
structure P2b (nP : Nat) where
  slot : Nat
  ballot : Ballot nP
  res : Except (Option (Ballot nP)) Unit
deriving DecidableEq, Repr, Lean.ToJson, Lean.FromJson

/-- The two clusters. -/
inductive PaxLoc | prop | acc
deriving DecidableEq, Repr

/-- Cluster sizes. -/
@[reducible] def paxMem (nP nA : Nat) : PaxLoc → Nat
  | .prop => nP
  | .acc => nA

/-- The a_log payload: `(checkpoint, log)` (checkpoint unused). -/
abbrev ALog (P : Type) (nP : Nat) := Option Nat × LogMap P nP

/-- **Covered, or collected**: a published `(checkpoint, log)` covers the
slot at the ballot, or its checkpoint exceeds the slot (the slot was
garbage-collected by `reduce_watermark` — decided and executed). The
shape every coverage face takes under GC (D71). -/
abbrev LogCoversOrGC {P : Type} [DecidableEq P] {nP : Nat} (lg : ALog P nP)
    (slot : Nat) (b : Ballot nP) : Prop :=
  LogCovers lg.2 slot b ∨ ∃ c, lg.1 = some c ∧ slot < c

/-- All accepted log entries of a batch of logs, pooled (Rust's
`flatten_unordered` of the logs) — the data vocabulary a consumer reads a
recovered view in. -/
def rcEntries {P : Type} [DecidableEq P] {nP : Nat} (logs : Multiset (ALog P nP)) :
    Multiset (Nat × LogValue P nP) :=
  logs.bind (fun lg => lg.2.entries)

/-- The running `.max()` dominates its seed. -/
theorem Multiset.foldl_maxStep_dom {α : Type _} [LinearOrder α] (m : Multiset α) :
    ∀ (y : α), ∃ r, Multiset.foldl maxStep (some y) m = some r ∧ y ≤ r := by
  induction m using Multiset.induction_on with
  | empty => intro y; exact ⟨y, rfl, le_refl _⟩
  | cons b m ih =>
    intro y
    rw [Multiset.foldl_cons]
    obtain ⟨r, hr, hle⟩ := ih (Max.max y b)
    exact ⟨r, hr, le_trans (le_max_left _ _) hle⟩

theorem Multiset.foldl_maxStep_ge {α : Type _} [LinearOrder α] (m : Multiset α) :
    ∀ (s : Option α) {x : α}, x ∈ m → ∃ r, Multiset.foldl maxStep s m = some r ∧ x ≤ r := by
  induction m using Multiset.induction_on with
  | empty => intro s x hx; exact absurd hx (Multiset.notMem_zero _)
  | cons a m ih =>
    intro s x hx
    rw [Multiset.foldl_cons]
    rcases Multiset.mem_cons.mp hx with rfl | hx'
    · obtain ⟨r, hr, hle⟩ := Multiset.foldl_maxStep_dom m (match s with | none => x | some y => Max.max y x)
      refine ⟨r, hr, le_trans ?_ hle⟩
      cases s with
      | none => exact le_refl _
      | some y => exact le_max_right _ _
    · exact ih _ hx'

/-- `p_max_slot` with the B4 fix: the champions' max slot, lifted to the
view's checkpoint under `.guarded` (`wmJoin`: none-bottomed max). -/
def rcMaxSlot (variant : PaxosVariant) (ms C : Option Nat) : Option Nat :=
  if variant.rebaseAboveCheckpoint then wmJoin ms C else ms

@[simp] theorem rcMaxSlot_none_none (variant : PaxosVariant) :
    rcMaxSlot variant none none = none := by
  unfold rcMaxSlot; split_ifs <;> rfl

theorem rcMaxSlot_ge_left (variant : PaxosVariant) {ms : Option Nat} (C : Option Nat) {a : Nat}
    (h : ms = some a) : ∃ mm, rcMaxSlot variant ms C = some mm ∧ a ≤ mm := by
  subst h
  unfold rcMaxSlot
  split_ifs
  · cases C with
    | none => exact ⟨a, rfl, le_refl _⟩
    | some c => exact ⟨max a c, rfl, le_max_left _ _⟩
  · exact ⟨a, rfl, le_refl _⟩

theorem rcMaxSlot_ge_ckpt {variant : PaxosVariant} (hv : variant.rebaseAboveCheckpoint = true)
    (ms : Option Nat) {C : Option Nat} {c : Nat} (h : C = some c) :
    ∃ mm, rcMaxSlot variant ms C = some mm ∧ c ≤ mm := by
  subst h
  unfold rcMaxSlot
  rw [if_pos hv]
  cases ms with
  | none => exact ⟨c, rfl, le_refl _⟩
  | some a => exact ⟨max a c, rfl, le_max_right _ _⟩

theorem rcMaxSlot_eq_none {variant : PaxosVariant} {ms C : Option Nat}
    (h : rcMaxSlot variant ms C = none) : ms = none := by
  unfold rcMaxSlot at h
  split_ifs at h with hv
  · cases ms with
    | none => rfl
    | some a => cases C <;> cases h
  · exact h


/-! ## The once-per-ballot register (FINDINGS B1, B2)

Both bugfixes are the same `use::state` register: a block FIRES at most
once per ballot, remembering the last ballot it fired at (B1: send each
ballot's P1a once; B2: recommit/rebase once per ballot, on becoming
leader). `OnceInv` is that register's loop invariant, in loop-invariant
normal form over the block's history (`Trace.hist`: each consumed input
paired with its emission); `fire`/`hold` are the two tick obligations. -/

/-- **Once per ballot**: the register `ra` names the last ballot the block
fired at. Over the history (inputs `ι` with ballot `bal`, emissions `β`
that `fired` or not), under the guarded flag `ro` (the faithful variant
claims nothing): no register, no fire; the register names a fire's
ballot and num-dominates every fire; two fires carry distinct ballots. -/
structure OnceInv {ι β : Type} {nP : Nat} (bal : ι → Ballot nP) (fired : β → Prop)
    (ro : Bool) (ra : Option (Ballot nP)) (hist : List (ι × β)) : Prop where
  /-- (guarded) No register, no fire yet. -/
  reg_none : ro = true → ra = none → ∀ p ∈ hist, ¬ fired p.2
  /-- (guarded) The register names a fire's ballot… -/
  reg_some : ro = true → ∀ r, ra = some r → ∃ p ∈ hist, fired p.2 ∧ bal p.1 = r
  /-- (guarded) …and num-dominates every fire's ballot. -/
  reg_dom : ro = true → ∀ r, ra = some r → ∀ p ∈ hist, fired p.2 → (bal p.1).num ≤ r.num
  /-- (guarded) **Once per ballot**: two fires carry distinct ballots. -/
  once : ro = true → List.Pairwise
    (fun p q : ι × β => fired p.2 → fired q.2 → bal p.1 ≠ bal q.1) hist

namespace OnceInv

variable {ι β : Type} {nP : Nat} {bal : ι → Ballot nP} {fired : β → Prop} {ro : Bool}

/-- The empty run: no register, no history. -/
theorem init {inp : List ι} : OnceInv bal fired ro none (Trace.hist inp []) := by
  simp only [Trace.hist, List.zip_nil_right]
  exact ⟨fun _ _ p hp => absurd hp (List.not_mem_nil), fun _ r h => (nomatch h),
    fun _ r h => (nomatch h), fun _ => List.Pairwise.nil⟩

/-- **FIRE**: the tick fires, on an input whose ballot the guarded register
does not name — the register names it after. The history's inputs are
one proposer's (owned) and num-ascending (wire facts), and the tick's
input is read at the history's length. -/
theorem fire {inp : List ι} {out : List β} {n : Nat} {x : ι} {e : β}
    {ra : Option (Ballot nP)}
    (hx : inp[n]? = some x) (hlen : out.length = n)
    (hown : ∀ y ∈ inp, (bal y).proposerId = (bal x).proposerId)
    (hmono : List.Pairwise (fun a b : ι => (bal a).num ≤ (bal b).num) inp)
    (ih : OnceInv bal fired ro ra (Trace.hist inp out))
    (hguard : ro = true → ra ≠ some (bal x)) (hfired : fired e) :
    OnceInv bal fired ro (some (bal x)) (Trace.hist inp (out ++ [e])) := by
  -- earlier ticks' ballots are num-below this tick's
  have hnum_le : ∀ p ∈ Trace.hist inp out, (bal p.1).num ≤ (bal x).num :=
    fun p hp => Trace.hist_mem_pairwise? hx hlen hmono hp
  -- no earlier fire carries this ballot: the register num-dominates every
  -- fire and sits below this ballot, and a ballot is its number plus its
  -- owner
  have hnew : ro = true → ∀ p ∈ Trace.hist inp out, fired p.2 → bal p.1 ≠ bal x := by
    intro hro p hp hfp heq
    cases hra0 : ra with
    | none => exact ih.reg_none hro hra0 p hp hfp
    | some r =>
      have h1 := ih.reg_dom hro r hra0 p hp hfp
      obtain ⟨q, hq, -, hqb⟩ := ih.reg_some hro r hra0
      have h2 := hnum_le q hq
      rw [hqb] at h2
      rw [heq] at h1
      have hown_r : r.proposerId = (bal x).proposerId := by
        rw [← hqb]
        exact hown _ (Trace.hist_mem_input hq)
      exact hguard hro (by rw [hra0, Ballot.eq_of_num_owner (Nat.le_antisymm h2 h1) hown_r])
  refine ⟨?_, ?_, ?_, ?_⟩ <;>
    simp only [Trace.hist_snoc? hx hlen, forall_mem_snoc, exists_mem_snoc, pairwise_snoc]
  · intro _ h
    cases h
  · intro _ r hr
    exact Or.inr ⟨hfired, Option.some.inj hr⟩
  · intro _ r hr
    obtain rfl := Option.some.inj hr
    exact ⟨fun p hp _ => hnum_le p hp, fun _ => Nat.le_refl _⟩
  · intro hro
    exact ⟨ih.once hro, fun p hp hfp _ => hnew hro p hp hfp⟩

/-- **HOLD**: the tick does not fire; the register carries. -/
theorem hold {inp : List ι} {out : List β} {n : Nat} {x : ι} {e : β}
    {ra : Option (Ballot nP)}
    (hx : inp[n]? = some x) (hlen : out.length = n)
    (ih : OnceInv bal fired ro ra (Trace.hist inp out)) (hnot : ¬ fired e) :
    OnceInv bal fired ro ra (Trace.hist inp (out ++ [e])) := by
  refine ⟨?_, ?_, ?_, ?_⟩ <;>
    simp only [Trace.hist_snoc? hx hlen, forall_mem_snoc, exists_mem_snoc, pairwise_snoc]
  · intro hro hra
    exact ⟨ih.reg_none hro hra, hnot⟩
  · intro hro r hr
    exact Or.inl (ih.reg_some hro r hr)
  · intro hro r hr
    exact ⟨ih.reg_dom hro r hr, fun h => absurd h hnot⟩
  · intro hro
    exact ⟨ih.once hro, fun _ _ _ hfe => absurd hfe hnot⟩

end OnceInv

/-! ## The leader wires' discipline -/

/-- **What a proposer's ballot / leading / view wires obey** — exactly
`leader_election`'s guarantee on them (`LEEnsures.discipline`) and
exactly what `sequence_payload` requires of them, read per tick
(`[t]?`). The composition point of the two halves of Paxos. -/
structure LeaderDiscipline (nP : Nat) (P : Type) [DecidableEq P]
    (pb : Fin nP → Trace (Ballot nP)) (pl : Fin nP → Trace Bool)
    (p1bs : Fin nP → Trace (Multiset (ALog P nP))) : Prop where
  /-- Every realized ballot is its member's own. -/
  own : ∀ (i : Fin nP), ∀ b ∈ pb i, (b : Ballot nP).proposerId = i
  /-- Ballot numbers only ascend along the tick trace (`p_ballot_calc`'s
  register only jumps up). -/
  mono : ∀ (i : Fin nP), List.Pairwise (fun (a b : Ballot nP) => a.num ≤ b.num) (pb i)
  /-- Leader ticks see nonempty views (a full quorum bucket). -/
  lead_ne : ∀ (i : Fin nP) {t : Nat} {v : Multiset (ALog P nP)},
    (pl i)[t]? = some true → (p1bs i)[t]? = some v → v ≠ 0
  /-- Same-ballot leader ticks see the same view — frozen quorum
  buckets. -/
  pinned : ∀ (i : Fin nP) {t t' : Nat} {b b' : Ballot nP}
    {v v' : Multiset (ALog P nP)},
    (pl i)[t]? = some true → (pl i)[t']? = some true →
    (pb i)[t]? = some b → (pb i)[t']? = some b' → b.num = b'.num →
    (p1bs i)[t]? = some v → (p1bs i)[t']? = some v' → v = v'
  /-- **The reign of a ballot**: every leader tick carrying `b` sits in a
  contiguous stretch of `b`-ticks whose first tick is fresh (did not lead
  the tick before — `defer_tick`'s read), before which no leader tick
  carried `b` (FINDINGS D21: a reign is one ballot). -/
  reign : ∀ (i : Fin nP) {t : Nat} {b : Ballot nP},
    (pl i)[t]? = some true → (pb i)[t]? = some b →
    ∃ t₀, t₀ ≤ t ∧ (pl i)[t₀]? = some true ∧ (pb i)[t₀]? = some b
      ∧ (false :: pl i)[t₀]? = some false
      ∧ (∀ u, t₀ ≤ u → u ≤ t → (pb i)[u]? = some b)
      ∧ (∀ u, u < t₀ → (pl i)[u]? = some true → (pb i)[u]? ≠ some b)

/-! ## The recommit champion fold (paxos.rs:617–640), literally (D71)

The per-slot `fold` of `recommit_after_leader_election`, as written:
`(count, champion)` — an entry with the champion's value bumps the
count; a strictly higher ballot takes the championship, resetting the
count to one when the value changes. The fold is NOT commutative
(FINDINGS D71's X·Y·Z: two orders of one multiset, opposite skip /
recommit decisions); the program realizes the tick's order by a
`tassume_ordering` decision and the facts below hold for EVERY order:
the champion is one of the fold's entries, dominates them all in ballot,
and the count never overshoots the entries carrying its value. -/

/-- The closure of paxos.rs:617–640. (When the higher ballot's value is
the champion's, the Rust rewrites only the ballot — the entry it keeps
IS the new entry.) -/
def rcStep {P : Type} [DecidableEq P] {nP : Nat}
    (acc : Nat × Option (LogValue P nP)) (e : LogValue P nP) :
    Nat × Option (LogValue P nP) :=
  match acc.2 with
  | none => (1, some e)
  | some cur =>
    if cur.ballot.blt e.ballot then
      (if e.value = cur.value then (acc.1 + 1, some e) else (1, some e))
    else
      (if e.value = cur.value then (acc.1 + 1, some cur) else (acc.1, some cur))

/-- The fold's invariant over the entries consumed so far. -/
def RCInv {P : Type} [DecidableEq P] {nP : Nat}
    (acc : Nat × Option (LogValue P nP)) (l : List (LogValue P nP)) : Prop :=
  match acc.2 with
  | none => l = []
  | some ch => ch ∈ l ∧ (∀ e ∈ l, e.ballot ≤ ch.ballot)
      ∧ acc.1 ≤ (l.filter (fun e => e.value = ch.value)).length

theorem rcStep_inv {P : Type} [DecidableEq P] {nP : Nat}
    {acc : Nat × Option (LogValue P nP)} {l : List (LogValue P nP)}
    (h : RCInv acc l) (e : LogValue P nP) : RCInv (rcStep acc e) (l ++ [e]) := by
  obtain ⟨n, ch?⟩ := acc
  cases ch? with
  | none =>
    simp only [RCInv] at h
    subst h
    simp [rcStep, RCInv]
  | some cur =>
    obtain ⟨hin, hdom, hcnt⟩ := h
    have hfilt_same : ∀ v, (l ++ [e]).filter (fun x => x.value = v)
        = l.filter (fun x => x.value = v) ++ [e].filter (fun x => x.value = v) :=
      fun v => List.filter_append _ _
    simp only [rcStep]
    by_cases hlt : cur.ballot.blt e.ballot = true
    · rw [if_pos hlt]
      have hlt' : cur.ballot < e.ballot := Ballot.lt_iff_blt.mpr hlt
      by_cases hsame : e.value = cur.value
      · rw [if_pos hsame]
        refine ⟨List.mem_append_right _ (List.mem_singleton_self _), ?_, ?_⟩
        · intro x hx
          rcases List.mem_append.mp hx with hx | hx
          · exact le_trans (hdom x hx) (le_of_lt hlt')
          · rw [List.mem_singleton.mp hx]
        · rw [hfilt_same, List.length_append]
          have : [e].filter (fun x => x.value = e.value) = [e] := by simp
          rw [this, List.length_singleton]
          have hl : l.filter (fun x => x.value = e.value) = l.filter (fun x => x.value = cur.value) := by
            rw [hsame]
          rw [hl]
          omega
      · rw [if_neg hsame]
        refine ⟨List.mem_append_right _ (List.mem_singleton_self _), ?_, ?_⟩
        · intro x hx
          rcases List.mem_append.mp hx with hx | hx
          · exact le_trans (hdom x hx) (le_of_lt hlt')
          · rw [List.mem_singleton.mp hx]
        · rw [hfilt_same, List.length_append]
          have : [e].filter (fun x => x.value = e.value) = [e] := by simp
          rw [this, List.length_singleton]
          omega
    · rw [if_neg hlt]
      have hle : e.ballot ≤ cur.ballot := by
        rcases le_or_gt e.ballot cur.ballot with h | h
        · exact h
        · exact absurd (Ballot.lt_iff_blt.mp h) hlt
      by_cases hsame : e.value = cur.value
      · rw [if_pos hsame]
        refine ⟨List.mem_append_left _ hin, ?_, ?_⟩
        · intro x hx
          rcases List.mem_append.mp hx with hx | hx
          · exact hdom x hx
          · rw [List.mem_singleton.mp hx]; exact hle
        · rw [hfilt_same, List.length_append]
          have : [e].filter (fun x => x.value = cur.value) = [e] := by simp [hsame]
          rw [this, List.length_singleton]
          omega
      · rw [if_neg hsame]
        refine ⟨List.mem_append_left _ hin, ?_, ?_⟩
        · intro x hx
          rcases List.mem_append.mp hx with hx | hx
          · exact hdom x hx
          · rw [List.mem_singleton.mp hx]; exact hle
        · rw [hfilt_same, List.length_append]
          exact le_trans hcnt (Nat.le_add_right _ _)

theorem rcFold_inv_aux {P : Type} [DecidableEq P] {nP : Nat} :
    ∀ (l : List (LogValue P nP)) {acc : Nat × Option (LogValue P nP)} {l₀ : List (LogValue P nP)},
      RCInv acc l₀ → RCInv (l.foldl rcStep acc) (l₀ ++ l)
  | [], _, _, h => by simpa using h
  | e :: l, acc, l₀, h => by
    rw [List.foldl_cons]
    have := rcFold_inv_aux l (rcStep_inv h e)
    rwa [List.append_assoc, List.singleton_append] at this

/-- The fold over a slot's entries, from the seed `(0, None)`: its
invariant over the whole list. -/
theorem rcFold_inv {P : Type} [DecidableEq P] {nP : Nat} (l : List (LogValue P nP)) :
    RCInv (l.foldl rcStep (0, none)) l := by
  have := rcFold_inv_aux l (acc := (0, none)) (l₀ := []) rfl
  rwa [List.nil_append] at this

/-- **The champion** of a nonempty slot: one of its entries, dominating
every entry's ballot, with the count bounded by the entries carrying its
value. -/
theorem rcFold_champ {P : Type} [DecidableEq P] {nP : Nat} {l : List (LogValue P nP)}
    {cnt : Nat} {ch : LogValue P nP} (h : l.foldl rcStep (0, none) = (cnt, some ch)) :
    ch ∈ l ∧ (∀ e ∈ l, e.ballot ≤ ch.ballot)
      ∧ cnt ≤ (l.filter (fun e => e.value = ch.value)).length := by
  have hinv := rcFold_inv l
  rw [h] at hinv
  exact hinv

/-- An empty champion only for an empty slot (so every key of the fold's
map carries a champion). -/
theorem rcFold_none {P : Type} [DecidableEq P] {nP : Nat} {l : List (LogValue P nP)}
    {cnt : Nat} (h : l.foldl rcStep (0, none) = (cnt, none)) : l = [] := by
  have hinv := rcFold_inv l
  rw [h] at hinv
  exact hinv

/-- The keyed fold's read, on a realized (ordered) tick batch: a key is
present iff it occurs, and its value is the per-key list fold. -/
theorem KeyedMap.get_ofFold_ofList {K V A : Type} [DecidableEq K] [DecidableEq V] [DecidableEq A]
    (g : A → V → A) (init : A) (Pre : V → V → Prop) [DecidableRel Pre]
    (ok : FoldOk .totalOrder .exactlyOnce Pre g) (l : List (K × V)) (k : K) :
    (KeyedMap.ofFold g init Pre ok (KeyedPool.ofList l)).get k
      = if k ∈ l.map Prod.fst then some ((keyVals k l).foldl g init) else none := by
  show (if k ∈ (l.map Prod.fst).toFinset then some ((keyVals k l).foldl g init) else none) = _
  simp only [List.mem_toFinset]

/-- Inserting is a permutation step (as multisets). -/
theorem coe_insertSorted {K V : Type} [LinearOrder K] :
    ∀ (l : List (K × V)) (e : K × V),
      ((insertSorted l e : List (K × V)) : Multiset (K × V)) = e ::ₘ ↑l
  | [], e => rfl
  | x :: l, e => by
    unfold insertSorted
    split_ifs
    · rfl
    · rw [← Multiset.cons_coe, coe_insertSorted l e, ← Multiset.cons_coe, Multiset.cons_swap]

/-- The insert fold's content is exactly the batch's (as a multiset). -/
theorem coe_foldl_insertSorted {K V : Type} [LinearOrder K] :
    ∀ (l acc : List (K × V)),
      ((l.foldl insertSorted acc : List (K × V)) : Multiset (K × V)) = ↑acc + ↑l
  | [], acc => by simp
  | e :: l, acc => by
    rw [List.foldl_cons, coe_foldl_insertSorted l, coe_insertSorted, ← Multiset.cons_coe,
      ← Multiset.singleton_add, ← Multiset.singleton_add]
    exact add_left_comm ({e} : Multiset (K × V)) (↑acc) (↑l)

theorem coe_foldlOn_insertSorted {K V : Type} [LinearOrder K] [DecidableEq K] [DecidableEq V]
    (m : Multiset (K × V)) (hm : PairwiseM NoKeyOverlap m) :
    ((Multiset.foldlOn NoKeyOverlap insertSorted [] insertSorted_foldOk m : List (K × V))
      : Multiset (K × V)) = m := by
  induction m using Quot.inductionOn with
  | h l =>
    change ((Multiset.foldlOn NoKeyOverlap insertSorted [] insertSorted_foldOk (↑l : Multiset (K × V))
      : List (K × V)) : Multiset (K × V)) = (↑l : Multiset (K × V))
    rw [Multiset.foldlOn_coe _ _ _ _ (PairwiseM.coe.mp hm), coe_foldl_insertSorted]
    simp

/-- The lifted order IS `ble`. -/
theorem Ballot.le_iff_ble {nP : Nat} {a b : Ballot nP} : a ≤ b ↔ a.ble b = true :=
  Ballot.ble_iff_key.symm

/-- The running `.max()` of a multiset is one of its elements (or the seed). -/
theorem Multiset.foldl_maxStep_mem {α : Type _} [LinearOrder α] (m : Multiset α) :
    ∀ (s : Option α) (c : α), Multiset.foldl maxStep s m = some c → c ∈ m ∨ s = some c := by
  induction m using Multiset.induction_on with
  | empty => intro s c h; exact Or.inr h
  | cons b m ih =>
    intro s c h
    rw [Multiset.foldl_cons] at h
    rcases ih _ c h with hm | hs
    · exact Or.inl (Multiset.mem_cons_of_mem hm)
    · cases s with
      | none =>
        simp only [maxStep, Option.some.injEq] at hs
        exact Or.inl (hs ▸ Multiset.mem_cons_self _ _)
      | some y =>
        simp only [maxStep, Option.some.injEq] at hs
        rcases max_choice y b with hy | hb
        · exact Or.inr (by rw [← hs, hy])
        · exact Or.inl (by rw [← hs, hb]; exact Multiset.mem_cons_self _ _)

/-- The keys of a keyed map's entries, once each. -/
theorem KeyedMap.entries_map_fst_nodup {K V : Type} [DecidableEq K] (m : KeyedMap K V) :
    (m.entries.map Prod.fst).Nodup := by
  unfold KeyedMap.entries
  rw [Multiset.map_filterMap]
  refine Multiset.Nodup.filterMap _ (fun a a' b ha ha2 => ?_) m.keys.nodup
  obtain ⟨x, hx, rfl⟩ := Option.map_eq_some_iff.mp (Option.mem_def.mp ha)
  obtain ⟨y, -, rfl⟩ := Option.map_eq_some_iff.mp hx
  obtain ⟨x', hx', hb⟩ := Option.map_eq_some_iff.mp (Option.mem_def.mp ha2)
  obtain ⟨y', -, rfl⟩ := Option.map_eq_some_iff.mp hx'
  exact hb.symm

end Hydro
