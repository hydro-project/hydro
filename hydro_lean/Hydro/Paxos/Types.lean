import Hydro.Sem
import Hydro.Trace
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

/-- Rust `Ballot { num: u32, proposer_id: ClusterId<Proposer> }`. -/
structure Ballot (nP : Nat) where
  num : Nat
  proposerId : Fin nP
deriving DecidableEq, Repr

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
deriving DecidableEq, Repr

/-- The acceptor log: slot-keyed, unique keys by construction. -/
abbrev LogMap (P : Type) (nP : Nat) := List (Nat × LogValue P nP)

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

/-! ## The canonical log view

paxos.rs:851–864's `reduce_watermark` merge (`manual_proof!(max by
ballot (TODO: not if two entries with same ballot, need assume))`) is
not commutative when two entries share a slot **and** a ballot with
different values. The model keeps the accumulated entry *multiset*
(trivially commutative and monotone) and computes the log as a pure
canonical view: per slot, the max-ballot entry, with ties resolved by a
commutative consensus fold — conflicting values degrade to `none`,
which provably never fires under the slot-functional input contract
(the Rust `assume`, made checkable). -/

/-- Consensus witness for the tie-break: what the max-ballot entries of
one slot agree on. -/
inductive ValWitness (P : Type) where
  | empty
  | one (v : Option P)
  | conflict
deriving DecidableEq, Repr

namespace ValWitness

variable {P : Type} [DecidableEq P]

def step : ValWitness P → Option P → ValWitness P
  | .empty, v => .one v
  | .one w, v => if v = w then .one w else .conflict
  | .conflict, _ => .conflict

theorem step_comm (s : ValWitness P) (x y : Option P) :
    step (step s x) y = step (step s y) x := by
  cases s with
  | empty =>
    show (if y = x then ValWitness.one x else .conflict)
      = (if x = y then ValWitness.one y else .conflict)
    by_cases h : x = y
    · subst h
      rfl
    · rw [if_neg (fun hc => h hc.symm), if_neg h]
  | one w =>
    by_cases hx : x = w <;> by_cases hy : y = w <;>
      simp [step, hx, hy]
  | conflict => rfl

end ValWitness

/-- The agreed value of one slot's max-ballot entries (`none` on
conflict — unreachable under the slot-functional contract). -/
def valOf {P : Type} [DecidableEq P] (vals : Multiset (Option P)) :
    Option P :=
  match @Multiset.foldl _ _ ValWitness.step
      ⟨fun s x y => ValWitness.step_comm s x y⟩ .empty vals with
  | .one v => v
  | _ => none

private theorem valOf_one_of_const {P : Type} [DecidableEq P]
    {v : Option P} :
    ∀ (vs : Multiset (Option P)), (∀ x ∈ vs, x = v) →
      @Multiset.foldl _ _ ValWitness.step
          ⟨fun s x y => ValWitness.step_comm s x y⟩ (.one v) vs
        = .one v := by
  intro vs
  induction vs using Multiset.induction_on with
  | empty => intro _; rfl
  | cons x m ih =>
    intro hconst
    rw [Multiset.foldl_cons]
    rw [show ValWitness.step (.one v) x = .one v by
      rw [hconst x (Multiset.mem_cons_self ..)]
      show (if v = v then ValWitness.one v else .conflict) = _
      rw [if_pos rfl]]
    exact ih (fun y hy => hconst y (Multiset.mem_cons_of_mem hy))


theorem valOf_const {P : Type} [DecidableEq P] {v : Option P}
    (vs : Multiset (Option P))
    (hne : vs ≠ 0) (hconst : ∀ x ∈ vs, x = v) : valOf vs = v := by
  obtain ⟨x, hx⟩ := Multiset.exists_mem_of_ne_zero hne
  obtain ⟨m, rfl⟩ := Multiset.exists_cons_of_mem hx
  unfold valOf
  rw [Multiset.foldl_cons]
  rw [show ValWitness.step .empty x = .one v by
    rw [hconst x (Multiset.mem_cons_self ..)]
    rfl]
  rw [valOf_one_of_const _
    (fun y hy => hconst y (Multiset.mem_cons_of_mem hy))]





/-- The canonical per-slot max-ballot log view of the accumulated entry
multiset (slots in ascending order; a pure function of the multiset, so
batch-boundary and arrival-order nondeterminism cannot leak). -/
def logView {P : Type} [DecidableEq P] {nP : Nat}
    (entries : Multiset (Nat × LogValue P nP)) : LogMap P nP :=
  ((entries.map Prod.fst).toFinset.sort (· ≤ ·)).filterMap (fun slot =>
    let sub := (entries.filter (fun e => e.1 = slot)).map Prod.snd
    match Ballot.maxFoldBatch none (sub.map (·.ballot)) with
    | some b =>
      some (slot,
        ⟨b, valOf ((sub.filter (fun lv => lv.ballot = b)).map (·.value))⟩)
    | none => none)


/-- Coverage of a slot at (or above) a ballot. -/
def LogCovers {P : Type} [DecidableEq P] {nP : Nat} (log : LogMap P nP)
    (slot : Nat) (b : Ballot nP) : Prop :=
  ∃ e : LogValue P nP, (slot, e) ∈ log ∧ b.ble e.ballot = true

/-- The canonical view covers every accumulated entry: `logView` keeps,
per slot, an entry at the slot's max ballot. -/
theorem logView_covers {P : Type} [DecidableEq P] {nP : Nat}
    (entries : Multiset (Nat × LogValue P nP)) {slot : Nat}
    {lv : LogValue P nP} (h : (slot, lv) ∈ entries) :
    LogCovers (logView entries) slot lv.ballot := by
  -- our slot survives to the sorted slot list
  have hslot : slot ∈ ((entries.map Prod.fst).toFinset.sort (· ≤ ·)) := by
    rw [Finset.mem_sort, Multiset.mem_toFinset]
    exact Multiset.mem_map.mpr ⟨(slot, lv), h, rfl⟩
  -- our ballot sits in the slot's sub-multiset
  have hball : lv.ballot ∈ ((entries.filter
      (fun e => e.1 = slot)).map Prod.snd).map (·.ballot) :=
    Multiset.mem_map.mpr ⟨lv,
      Multiset.mem_map.mpr ⟨(slot, lv),
        Multiset.mem_filter.mpr ⟨h, rfl⟩, rfl⟩, rfl⟩
  obtain ⟨r, hr, hle⟩ := Ballot.maxFoldBatch_mem_ble none _ _ hball
  -- the filterMap keeps our slot, at the batch max
  refine ⟨⟨r, valOf ((((entries.filter (fun e => e.1 = slot)).map
      Prod.snd).filter (fun lv => lv.ballot = r)).map (·.value))⟩,
    ?_, hle⟩
  refine List.mem_filterMap.mpr ⟨slot, hslot, ?_⟩
  show (match Ballot.maxFoldBatch none (((entries.filter
      (fun e => e.1 = slot)).map Prod.snd).map (·.ballot)) with
    | some b => some (slot, (⟨b, valOf ((((entries.filter
        (fun e => e.1 = slot)).map Prod.snd).filter
        (fun lv => lv.ballot = b)).map (·.value))⟩ : LogValue P nP))
    | none => none)
    = some (slot, ⟨r, valOf ((((entries.filter (fun e => e.1 = slot)).map
      Prod.snd).filter (fun lv => lv.ballot = r)).map (·.value))⟩)
  rw [hr]


/-- Entry inversion for the canonical view: a `logView` entry's value is
the consensus of its slot's max-ballot group, and the group is
inhabited. -/
theorem logView_entry {P : Type} [DecidableEq P] {nP : Nat}
    (entries : Multiset (Nat × LogValue P nP)) {slot : Nat}
    {e : LogValue P nP} (h : (slot, e) ∈ logView entries) :
    e.value = valOf ((((entries.filter (fun x => x.1 = slot)).map
        Prod.snd).filter (fun lv => lv.ballot = e.ballot)).map
        (·.value))
    ∧ ∃ lv : LogValue P nP,
        (slot, lv) ∈ entries ∧ lv.ballot = e.ballot := by
  unfold logView at h
  obtain ⟨slot', hslot', hfn⟩ := List.mem_filterMap.mp h
  have hfn2 : (match Ballot.maxFoldBatch none
      ((((entries.filter (fun x => x.1 = slot')).map Prod.snd)).map
        (·.ballot)) with
    | some b =>
      some ((slot',
        ⟨b, valOf ((((entries.filter (fun x => x.1 = slot')).map
          Prod.snd).filter (fun lv => lv.ballot = b)).map (·.value))⟩)
        : Nat × LogValue P nP)
    | none => none) = some (slot, e) := hfn
  cases hmax : Ballot.maxFoldBatch none
      ((((entries.filter (fun x => x.1 = slot')).map Prod.snd)).map
        (·.ballot)) with
  | none =>
    rw [hmax] at hfn2
    cases hfn2
  | some b =>
    rw [hmax] at hfn2
    dsimp only at hfn2
    injection hfn2 with hfn'
    have hslot_eq : slot' = slot := congrArg Prod.fst hfn'
    subst hslot_eq
    have he : e = ⟨b, valOf ((((entries.filter
        (fun x => x.1 = slot')).map Prod.snd).filter
        (fun lv => lv.ballot = b)).map (·.value))⟩ :=
      (congrArg Prod.snd hfn').symm
    have hball : e.ballot = b := by rw [he]
    constructor
    · rw [hball]
      rw [he]
    · have hmem : b ∈ ((entries.filter (fun x => x.1 = slot')).map
          Prod.snd).map (·.ballot) :=
        Ballot.maxFoldBatch_mem_of_some hmax
      obtain ⟨lv, hlv, hlvb⟩ := Multiset.mem_map.mp hmem
      obtain ⟨x, hx, hxsnd⟩ := Multiset.mem_map.mp hlv
      have hx1 : x.1 = slot' := (Multiset.mem_filter.mp hx).2
      refine ⟨lv, ?_, by rw [hlvb, hball]⟩
      have hxeq : x = (slot', lv) := by
        cases x
        simp_all
      rw [← hxeq]
      exact Multiset.mem_of_le (Multiset.filter_le _ _) hx

/-- Coverage only grows with the accumulated entries. -/
theorem logView_covers_mono {P : Type} [DecidableEq P] {nP : Nat}
    {s t : Multiset (Nat × LogValue P nP)} (hst : s ≤ t)
    {slot : Nat} {b : Ballot nP} (h : LogCovers (logView s) slot b) :
    LogCovers (logView t) slot b := by
  obtain ⟨e, hmem, hble⟩ := h
  obtain ⟨-, lv, hlv, hlvb⟩ := logView_entry s hmem
  have hlvt : (slot, lv) ∈ t := Multiset.mem_of_le hst hlv
  obtain ⟨e', he', hble'⟩ := logView_covers t hlvt
  refine ⟨e', he', ?_⟩
  rw [hlvb] at hble'
  exact Ballot.ble_trans hble hble'

/-- **A view entry's value is its group's**: if every accumulated entry at
the view entry's `(slot, ballot)` carries the value `V`, the view entry
carries `V` (the `valOf` consensus on an agreeing, inhabited group) — and
some accumulated entry at that key exists. -/
theorem logView_entry_value {P : Type} [DecidableEq P] {nP : Nat}
    (entries : Multiset (Nat × LogValue P nP)) {slot : Nat}
    {e : LogValue P nP} (h : (slot, e) ∈ logView entries) :
    (∃ lv : LogValue P nP, (slot, lv) ∈ entries ∧ lv.ballot = e.ballot)
    ∧ ∀ V : Option P,
      (∀ lv : LogValue P nP, (slot, lv) ∈ entries → lv.ballot = e.ballot →
        lv.value = V) →
      e.value = V := by
  obtain ⟨hval, lv0, hlv0, hlv0b⟩ := logView_entry entries h
  refine ⟨⟨lv0, hlv0, hlv0b⟩, fun V hV => ?_⟩
  rw [hval]
  refine valOf_const _ ?_ ?_
  · intro habs
    have hlv0g : lv0 ∈ (((entries.filter (fun x => x.1 = slot)).map Prod.snd).filter
        (fun lv => lv.ballot = e.ballot)) :=
      Multiset.mem_filter.mpr
        ⟨Multiset.mem_map.mpr ⟨(slot, lv0), Multiset.mem_filter.mpr ⟨hlv0, rfl⟩, rfl⟩, hlv0b⟩
    have := Multiset.map_eq_zero.mp habs
    rw [this] at hlv0g
    cases hlv0g
  · intro x hx
    obtain ⟨w, hw, rfl⟩ := Multiset.mem_map.mp hx
    have hw1 := Multiset.of_mem_filter hw
    obtain ⟨y, hy, rfl⟩ := Multiset.mem_map.mp (Multiset.mem_of_le (Multiset.filter_le _ _) hw)
    have hy1 : y.1 = slot := (Multiset.mem_filter.mp hy).2
    have hy2 : y ∈ entries := Multiset.mem_of_le (Multiset.filter_le _ _) hy
    exact hV y.2 (by rw [← hy1]; exact hy2) hw1

/-- The p1b reply (paxos.rs:485–496): `Ok (checkpoint, log)` or
`Err max_ballot`. -/
structure P1b (P : Type) (nP : Nat) where
  ballot : Ballot nP
  res : Except (Option (Ballot nP)) (Option Nat × LogMap P nP)
deriving DecidableEq, Repr

/-- Rust `P2a { sender, ballot, slot, value }` (paxos.rs:64–70). -/
structure P2a (P : Type) (nP : Nat) where
  sender : Fin nP
  ballot : Ballot nP
  slot : Nat
  value : Option P
deriving DecidableEq, Repr

/-- The p2b ack (paxos.rs:875–890): key `(slot, ballot)`, `Ok` iff the
ballot is the acceptor's current max. -/
structure P2b (nP : Nat) where
  slot : Nat
  ballot : Ballot nP
  res : Except (Option (Ballot nP)) Unit
deriving DecidableEq, Repr

/-- The two clusters. -/
inductive PaxLoc | prop | acc
deriving DecidableEq, Repr

/-- Cluster sizes. -/
@[reducible] def paxMem (nP nA : Nat) : PaxLoc → Nat
  | .prop => nP
  | .acc => nA

/-- The a_log payload: `(checkpoint, log)` (checkpoint unused). -/
abbrev ALog (P : Type) (nP : Nat) := Option Nat × LogMap P nP

/-- `logView`'s slots are duplicate-free (one champion per slot). -/
theorem logView_keys_nodup {P : Type} [DecidableEq P] {nP : Nat}
    (entries : Multiset (Nat × LogValue P nP)) :
    ((logView entries).map Prod.fst).Nodup := by
  unfold logView
  refine List.Sublist.nodup
    (List.map_filterMap_sublist _ Prod.fst id ?_ _) ?_
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

/-- All accepted log entries of a batch of logs, pooled (Rust's
`flatten_unordered` of the logs) — the data vocabulary a consumer reads a
recovered view in. -/
def rcEntries {P : Type} [DecidableEq P] {nP : Nat} (logs : Multiset (ALog P nP)) :
    Multiset (Nat × LogValue P nP) :=
  logs.bind (fun lg => Multiset.ofList lg.2)

/-- The number of accepted entries agreeing with a value at one slot
(paxos.rs:618's incremental `count`, made canonical). -/
def rcCount {P : Type} [DecidableEq P] {nP : Nat} (ents : Multiset (Nat × LogValue P nP))
    (slot : Nat) (v : Option P) : Nat :=
  (ents.filter (fun e => e.1 = slot ∧ e.2.value = v)).card

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

end Hydro
