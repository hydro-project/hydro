import Mathlib.Data.Multiset.Basic
import Mathlib.Data.Multiset.AddSub
import Mathlib.Data.Multiset.MapFold
import Mathlib.Data.Multiset.Dedup
import Mathlib.Data.Multiset.Bind
import Mathlib.Data.Multiset.Sort
import Mathlib.Data.Finset.Basic
import Hydro.Order
import Hydro.SimpAttr

/-!
# Hydro · graded stream content (the anti-cheating core)

A stream's Rust markers — `Ordering ∈ {TotalOrder, NoOrder}` ×
`Retries ∈ {ExactlyOnce, AtLeastOnce}` — grade what a consumer may
observe. The guard principle: **whatever the marker refuses to promise
is unobservable by type**, via a quotient whose classes collect exactly
the executions the marker identifies:

| grade | carrier | identified executions | consumer obligation |
|---|---|---|---|
| `TotalOrder, ExactlyOnce` | `List α` | none | — |
| `NoOrder, ExactlyOnce` | `Multiset α` (= `List/Perm`) | reorderings | commutativity |
| `TotalOrder, AtLeastOnce` | `StutterSeq α` (= `List/≈stutter`) | consecutive re-deliveries | consecutive idempotence |
| `NoOrder, AtLeastOnce` | `RetryPool α` (= `Multiset/=support`) | reorderings + re-deliveries | commutativity + idempotence |

Every class has a **normalized representative** (the list itself / the
sorted content / the destuttered list / the support `Finset`) — but the
normal form is only a *derived view*: actual executions live anywhere in
the class (with duplicates), and a consumer exists **only** as a
`Quotient.lift`, whose respect proof is exactly the obligation above. An
operator that mishandles duplicates or order does not verify wrongly —
it fails to typecheck. (Mathlib supplies the `Multiset`/`Finset` layers;
the mechanism itself is core `Quotient`.)
-/

namespace Hydro

/-- Rust's stream ordering marker. -/
inductive StrOrd where
  | totalOrder
  | noOrder
deriving DecidableEq, Inhabited

/-- Rust's stream retries marker (`hydro_lang` `Retries`). -/
inductive Retries where
  | exactlyOnce
  | atLeastOnce
deriving DecidableEq, Inhabited

/-! ## `TotalOrder + AtLeastOnce`: sequences modulo consecutive stutter

Ordered transport with re-delivery duplicates elements **in place**
(sequential stutters; a retry never jumps past other elements). -/

/-- Collapse consecutive duplicates (the normal form). -/
def destutter {α : Type _} [DecidableEq α] : List α → List α
  | [] => []
  | [x] => [x]
  | x :: y :: rest =>
    if x = y then destutter (y :: rest) else x :: destutter (y :: rest)

@[simp] theorem destutter_nil {α : Type _} [DecidableEq α] :
    destutter ([] : List α) = [] := rfl

theorem destutter_cons_cons {α : Type _} [DecidableEq α]
    (x y : α) (rest : List α) :
    destutter (x :: y :: rest)
      = if x = y then destutter (y :: rest)
        else x :: destutter (y :: rest) := rfl

/-- Destuttering keeps the head. -/
theorem destutter_cons_head {α : Type _} [DecidableEq α] (x : α) :
    ∀ (l : List α), ∃ t, destutter (x :: l) = x :: t
  | [] => ⟨[], rfl⟩
  | y :: rest => by
    rw [destutter_cons_cons]
    by_cases h : x = y
    · subst h
      obtain ⟨t, ht⟩ := destutter_cons_head x rest
      exact ⟨t, by rw [if_pos rfl]; exact ht⟩
    · exact ⟨destutter (y :: rest), by rw [if_neg h]⟩

/-- Stutter-collapse is prefix-stable: a longer arrival sequence
destutters to an extension. -/
theorem destutter_append_prefix {α : Type _} [DecidableEq α] :
    ∀ (v e : List α), destutter v <+: destutter (v ++ e)
  | [], _ => List.nil_prefix
  | [x], e => by
    cases e with
    | nil => exact List.prefix_refl _
    | cons y rest =>
      show destutter [x] <+: destutter (x :: y :: rest)
      obtain ⟨t, ht⟩ := destutter_cons_head x (y :: rest)
      rw [ht]
      exact ⟨t, rfl⟩
  | x :: y :: rest, e => by
    show destutter (x :: y :: rest) <+: destutter (x :: y :: (rest ++ e))
    rw [destutter_cons_cons, destutter_cons_cons]
    by_cases h : x = y
    · rw [if_pos h, if_pos h]
      exact destutter_append_prefix (y :: rest) e
    · rw [if_neg h, if_neg h]
      exact List.cons_prefix_cons.mpr
        ⟨rfl, destutter_append_prefix (y :: rest) e⟩

/-- Destuttering only drops elements. -/
theorem destutter_sublist {α : Type _} [DecidableEq α] :
    ∀ (l : List α), List.Sublist (destutter l) l
  | [] => List.Sublist.refl _
  | [x] => List.Sublist.refl _
  | x :: y :: rest => by
    rw [destutter_cons_cons]
    by_cases h : x = y
    · rw [if_pos h]
      exact (destutter_sublist (y :: rest)).cons x
    · rw [if_neg h]
      exact (destutter_sublist (y :: rest)).cons₂ x

/-- Executions identified by consecutive re-delivery. -/
def stutterSetoid (α : Type _) [DecidableEq α] : Setoid (List α) where
  r a b := destutter a = destutter b
  iseqv := ⟨fun _ => rfl, Eq.symm, Eq.trans⟩

/-- Ordered at-least-once content: what the receiver is *entitled to* —
the sequence up to consecutive duplication. Consumers exist only via
`StutterSeq.fold`-style lifts (obligation: consecutive idempotence). -/
def StutterSeq (α : Type _) [DecidableEq α] : Type _ :=
  Quotient (stutterSetoid α)

namespace StutterSeq

variable {α : Type _} [DecidableEq α]

/-- An arrival sequence, as its entitlement class. -/
def mk (l : List α) : StutterSeq α := Quotient.mk _ l

/-- The normalized representative (derived view — never the carrier). -/
def norm (s : StutterSeq α) : List α :=
  Quotient.lift destutter (fun _ _ h => by exact h) s

@[simp] theorem norm_mk (l : List α) : (mk l).norm = destutter l := rfl

/-- A consecutive-idempotent step ignores stutter: folding an execution
equals folding its normal form. -/
theorem foldl_destutter {σ : Type _} (g : σ → α → σ)
    (idem : ∀ s x, g (g s x) x = g s x) :
    ∀ (l : List α) (init : σ),
      l.foldl g init = (destutter l).foldl g init
  | [], _ => rfl
  | [x], _ => rfl
  | x :: y :: rest, init => by
    by_cases h : x = y
    · subst h
      show (x :: rest).foldl g (g init x) = _
      rw [show destutter (x :: x :: rest) = destutter (x :: rest) from by
        rw [destutter_cons_cons, if_pos rfl]]
      calc (x :: rest).foldl g (g init x)
          = rest.foldl g (g (g init x) x) := rfl
        _ = rest.foldl g (g init x) := by rw [idem]
        _ = (x :: rest).foldl g init := rfl
        _ = (destutter (x :: rest)).foldl g init :=
            foldl_destutter g idem (x :: rest) init
    · rw [show destutter (x :: y :: rest) = x :: destutter (y :: rest)
          from by
        rw [destutter_cons_cons, if_neg h]]
      show (y :: rest).foldl g (g init x) = _
      rw [foldl_destutter g idem (y :: rest) (g init x)]
      rfl

/-- **The consumer gate**: a fold exists only for consecutive-idempotent
steps — the respect proof *is* the obligation. -/
def fold {σ : Type _} (g : σ → α → σ) (init : σ)
    (idem : ∀ s x, g (g s x) x = g s x) (s : StutterSeq α) : σ :=
  Quotient.lift (fun l => l.foldl g init)
    (fun a b h => by
      have h' : destutter a = destutter b := h
      rw [foldl_destutter g idem a init, foldl_destutter g idem b init, h'])
    s

@[simp] theorem fold_mk {σ : Type _} (g : σ → α → σ) (init : σ)
    (idem : ∀ s x, g (g s x) x = g s x) (l : List α) :
    fold g init idem (mk l) = l.foldl g init := rfl

/-- Growth: the entitlement extends (prefix of normal forms — new
elements arrive at the end; stutter never reorders). -/
def le (a b : StutterSeq α) : Prop := a.norm <+: b.norm

theorem le_refl (a : StutterSeq α) : le a a := List.prefix_refl _

theorem le_trans {a b c : StutterSeq α} (h₁ : le a b) (h₂ : le b c) :
    le a c := List.IsPrefix.trans h₁ h₂

/-- Appending arrivals grows the entitlement. -/
theorem le_mk_append (v e : List α) : le (mk v) (mk (v ++ e)) := by
  show destutter v <+: destutter (v ++ e)
  exact destutter_append_prefix v e

/-- The head of a destuttered list is the head of the list. -/
theorem destutter_head? : ∀ (l : List α), (destutter l).head? = l.head?
  | [] => rfl
  | [x] => rfl
  | x :: y :: rest => by
    rw [destutter_cons_cons]
    split
    · rename_i h; subst h
      rw [destutter_head? (x :: rest)]; rfl
    · rfl

/-- `.first()` of an at-least-once sequence (D91): stutter never changes the head, so the
class has one. -/
def head? (s : StutterSeq α) : Option α :=
  Quotient.lift List.head?
    (fun a b h => by
      have h' : destutter a = destutter b := h
      rw [← destutter_head? a, ← destutter_head? b, h']) s

@[simp] theorem head?_mk (l : List α) : (mk l).head? = l.head? := rfl

end StutterSeq

/-- Every non-empty sequence of `()` destutters to `[()]` (D91: the interval stream's
one class). -/
theorem destutter_replicate_unit : ∀ (k : Nat), destutter (List.replicate k ()) = if k = 0 then [] else [()]
  | 0 => rfl
  | 1 => rfl
  | k + 2 => by
    rw [List.replicate_succ, List.replicate_succ, destutter_cons_cons, if_pos rfl,
      ← List.replicate_succ, destutter_replicate_unit (k + 1)]
    simp

/-- A finite run of `()`s sits below the class `[()]`. -/
theorem stutter_le_unit (k : Nat) : StutterSeq.le (StutterSeq.mk (List.replicate k ())) (StutterSeq.mk [()]) := by
  show destutter (List.replicate k ()) <+: destutter [()]
  rw [destutter_replicate_unit]
  split
  · exact List.nil_prefix
  · exact List.prefix_refl _

/-! ## Conditional commutativity (D71, R3)

Rust's `commutative = manual_proof!(…)` is almost never a proof that the
closure commutes on *all* pairs: paxos.rs's log merge commutes only on
pairs that do not share a ballot with different values. The obligation
is therefore **conditional** — `hcomm : ∀ s x y, Pre x y → g (g s x) y =
g (g s y) x` for a program-chosen prerequisite `Pre` on pairs — and the
quotient fold exists for content whose pairs all satisfy `Pre`
(permutation-invariance of `List.foldl` needs commutativity only on the
list's own pairs: `List.Perm.foldl_eq'`). On content that violates the
prerequisite the fold returns its seed — the executable meaning of an
unjustified `manual_proof!`; every compiled `hydro def` proves the
pairwise fact at the site (`ghost pre`), so the branch is unreachable
there and only shows in raw `Values` runs of violating inputs (the
falsifier's). `Pre := fun _ _ => True` is today's unconditional
obligation. -/

/-- All pairs of a list satisfy `Pre` (both orders). -/
def PairwiseL {α : Type _} (Pre : α → α → Prop) (l : List α) : Prop :=
  ∀ x ∈ l, ∀ y ∈ l, Pre x y

instance {α : Type _} (Pre : α → α → Prop) [DecidableRel Pre] (l : List α) :
    Decidable (PairwiseL Pre l) :=
  inferInstanceAs (Decidable (∀ x ∈ l, ∀ y ∈ l, Pre x y))

/-- All pairs of a multiset satisfy `Pre`. -/
def PairwiseM {α : Type _} (Pre : α → α → Prop) (m : Multiset α) : Prop :=
  ∀ x ∈ m, ∀ y ∈ m, Pre x y

theorem PairwiseL.perm {α : Type _} {Pre : α → α → Prop} {l₁ l₂ : List α}
    (h : l₁.Perm l₂) : PairwiseL Pre l₁ ↔ PairwiseL Pre l₂ := by
  unfold PairwiseL
  simp only [h.mem_iff]

theorem PairwiseL.of_subset {α : Type _} {Pre : α → α → Prop} {l l' : List α}
    (h : l ⊆ l') (hp : PairwiseL Pre l') : PairwiseL Pre l :=
  fun x hx y hy => hp x (h hx) y (h hy)

theorem PairwiseL.true {α : Type _} (l : List α) :
    PairwiseL (fun _ _ => True) l := fun _ _ _ _ => trivial

theorem PairwiseM.coe {α : Type _} {Pre : α → α → Prop} {l : List α} :
    PairwiseM Pre (↑l : Multiset α) ↔ PairwiseL Pre l := Iff.rfl

theorem PairwiseM.of_le {α : Type _} {Pre : α → α → Prop} {m m' : Multiset α}
    (h : m ≤ m') (hp : PairwiseM Pre m') : PairwiseM Pre m :=
  fun x hx y hy => hp x (Multiset.mem_of_le h hx) y (Multiset.mem_of_le h hy)

theorem PairwiseM.true {α : Type _} (m : Multiset α) :
    PairwiseM (fun _ _ => True) m := fun _ _ _ _ => trivial

theorem PairwiseM.cons {α : Type _} {Pre : α → α → Prop} {a : α} {m : Multiset α}
    (h : PairwiseM Pre (a ::ₘ m)) : PairwiseM Pre m :=
  PairwiseM.of_le (Multiset.le_cons_self m a) h

/-- The order-free fold under conditional commutativity: the list fold of
any representative when every pair satisfies `Pre`, else the seed. -/
def Multiset.foldlOn {α σ : Type _} (Pre : α → α → Prop) [DecidableRel Pre]
    (g : σ → α → σ) (init : σ)
    (hcomm : ∀ s x y, Pre x y → g (g s x) y = g (g s y) x) (m : Multiset α) : σ :=
  Quot.liftOn m (fun l => if PairwiseL Pre l then l.foldl g init else init)
    (fun l₁ l₂ h => by
      have hp : PairwiseL Pre l₁ ↔ PairwiseL Pre l₂ := PairwiseL.perm h
      by_cases h1 : PairwiseL Pre l₁
      · rw [if_pos h1, if_pos (hp.mp h1)]
        exact h.foldl_eq' (fun x hx y hy z => hcomm z x y (h1 x hx y hy)) init
      · rw [if_neg h1, if_neg (fun h2 => h1 (hp.mpr h2))])

namespace Multiset

variable {α σ : Type _} (Pre : α → α → Prop) [DecidableRel Pre]
  (g : σ → α → σ) (init : σ) (hcomm : ∀ s x y, Pre x y → g (g s x) y = g (g s y) x)

/-- **The reader**: on content whose pairs satisfy `Pre`, the fold IS the
list fold of the representative. -/
theorem foldlOn_coe {l : List α} (hl : PairwiseL Pre l) :
    foldlOn Pre g init hcomm (↑l : Multiset α) = l.foldl g init := by
  show (if PairwiseL Pre l then l.foldl g init else init) = _
  rw [if_pos hl]

theorem foldlOn_zero : foldlOn Pre g init hcomm 0 = init := rfl

/-- **The refusal**: on content with a `Pre`-violating pair the fold is
its seed (the denotation's executable meaning of an unjustified
`manual_proof!` — unreachable in a compiled `hydro def`). -/
theorem foldlOn_of_not_pairwise {m : Multiset α} (h : ¬ PairwiseM Pre m) :
    foldlOn Pre g init hcomm m = init := by
  induction m using Quot.inductionOn with
  | h l =>
    have hl : ¬ PairwiseL Pre l := h
    show (if PairwiseL Pre l then l.foldl g init else init) = init
    rw [if_neg hl]

theorem foldlOn_cons {a : α} {m : Multiset α} (h : PairwiseM Pre (a ::ₘ m)) :
    foldlOn Pre g init hcomm (a ::ₘ m) = foldlOn Pre g (g init a) hcomm m := by
  induction m using Quot.inductionOn with
  | h l =>
    have hl : PairwiseL Pre (a :: l) := h
    show (if PairwiseL Pre (a :: l) then (a :: l).foldl g init else init)
      = (if PairwiseL Pre l then l.foldl g (g init a) else g init a)
    rw [if_pos hl, if_pos (PairwiseL.of_subset (List.subset_cons_self a l) hl)]
    rfl

/-- The unconditional obligation recovers Mathlib's fold. -/
theorem foldlOn_true (g : σ → α → σ) (init : σ)
    (hcomm : ∀ s x y, True → g (g s x) y = g (g s y) x) (m : Multiset α) :
    foldlOn (fun _ _ => True) g init hcomm m
      = @Multiset.foldl α σ g ⟨fun s x y => hcomm s x y trivial⟩ init m := by
  induction m using Quot.inductionOn with
  | h l =>
    show (if PairwiseL (fun _ _ => True) l then l.foldl g init else init) = _
    rw [if_pos (PairwiseL.true l)]
    rfl

end Multiset

/-! ## `NoOrder + AtLeastOnce`: multisets modulo retry multiplicity

Unordered transport with re-delivery: neither arrival order nor arrival
counts are promised. Executions in one class differ by duplication; the
normalized representative is the support `Finset` — a derived view. -/

/-- Executions identified by retry multiplicity. -/
def supportSetoid (α : Type _) : Setoid (Multiset α) where
  r a b := ∀ x, x ∈ a ↔ x ∈ b
  iseqv := ⟨fun _ _ => Iff.rfl,
    fun h x => (h x).symm,
    fun h₁ h₂ x => (h₁ x).trans (h₂ x)⟩

/-- Unordered at-least-once content: arrivals up to reordering **and**
retry multiplicity. Consumers exist only via lifts that handle every
duplicated execution in the class. -/
def RetryPool (α : Type _) : Type _ := Quotient (supportSetoid α)

namespace RetryPool

variable {α : Type _}

/-- An arrival multiset, as its entitlement class. -/
def mk (m : Multiset α) : RetryPool α := Quotient.mk _ m

/-- Membership *is* granted by the marker. -/
def Mem (x : α) (p : RetryPool α) : Prop :=
  Quotient.lift (fun m => x ∈ m)
    (fun a b h => by
      have h' : ∀ y, y ∈ a ↔ y ∈ b := h
      exact propext (h' x)) p

instance : Membership α (RetryPool α) := ⟨fun p x => Mem x p⟩

@[simp] theorem mem_mk (x : α) (m : Multiset α) :
    x ∈ mk m ↔ x ∈ m := Iff.rfl

/-- The normalized representative (derived view — never the carrier). -/
def support [DecidableEq α] (p : RetryPool α) : Finset α :=
  Quotient.lift Multiset.toFinset
    (fun a b h => by
      have h' : ∀ y, y ∈ a ↔ y ∈ b := h
      exact Finset.ext fun x => by
        rw [Multiset.mem_toFinset, Multiset.mem_toFinset]
        exact h' x) p

@[simp] theorem support_mk [DecidableEq α] (m : Multiset α) :
    support (mk m) = m.toFinset := rfl

/-- Conditionally commutative + idempotent steps ignore retry multiplicity
on content whose pairs satisfy `Pre`: folding an arrival list equals
folding its deduplication. -/
theorem foldl_dedup_of [DecidableEq α] {σ : Type _} (Pre : α → α → Prop)
    (g : σ → α → σ) (hcomm : ∀ s x y, Pre x y → g (g s x) y = g (g s y) x)
    (idem : ∀ s x, g (g s x) x = g s x) :
    ∀ (l : List α), PairwiseL Pre l → ∀ (init : σ),
      l.foldl g init = l.dedup.foldl g init
  | [], _, _ => rfl
  | x :: rest, hp, init => by
    have hrest : PairwiseL Pre rest :=
      PairwiseL.of_subset (List.subset_cons_self x rest) hp
    by_cases hx : x ∈ rest
    · rw [List.dedup_cons_of_mem hx, ← foldl_dedup_of Pre g hcomm idem rest hrest init]
      -- absorbing `x` first is a no-op: `x` recurs in `rest`; pull its
      -- other copy adjacent (commutativity on `rest`'s pairs) and
      -- collapse (idempotence)
      show rest.foldl g (g init x) = rest.foldl g init
      have hperm : rest.Perm (x :: rest.erase x) := List.perm_cons_erase hx
      have hcomm' : ∀ a ∈ rest, ∀ b ∈ rest, ∀ z, g (g z a) b = g (g z b) a :=
        fun a ha b hb z => hcomm z a b (hrest a ha b hb)
      rw [hperm.foldl_eq' hcomm' (g init x), hperm.foldl_eq' hcomm' init]
      show (rest.erase x).foldl g (g (g init x) x) = (rest.erase x).foldl g (g init x)
      rw [idem]
    · rw [List.dedup_cons_of_notMem hx]
      show rest.foldl g (g init x) = rest.dedup.foldl g (g init x)
      exact foldl_dedup_of Pre g hcomm idem rest hrest (g init x)

instance [DecidableEq α] (Pre : α → α → Prop) [DecidableRel Pre] (m : Multiset α) :
    Decidable (PairwiseM Pre m) :=
  inferInstanceAs (Decidable (∀ x ∈ m, ∀ y ∈ m, Pre x y))

/-- **The consumer gate**: a fold exists only for (conditionally)
commutative *and* idempotent steps — support-equal executions must fold
equal. On content violating `Pre`, the seed. -/
def foldOn [DecidableEq α] {σ : Type _} (Pre : α → α → Prop) [DecidableRel Pre]
    (g : σ → α → σ) (init : σ)
    (hcomm : ∀ s x y, Pre x y → g (g s x) y = g (g s y) x)
    (_idem : ∀ s x, g (g s x) x = g s x) (p : RetryPool α) : σ :=
  Quotient.lift
    (fun m => if PairwiseM Pre m then Multiset.foldlOn Pre g init hcomm m.dedup else init)
    (fun a b h => by
      have h' : ∀ y, y ∈ a ↔ y ∈ b := h
      have hp : PairwiseM Pre a ↔ PairwiseM Pre b := by
        unfold PairwiseM
        simp only [h']
      have hd : a.dedup = b.dedup :=
        ((Multiset.nodup_dedup a).ext (Multiset.nodup_dedup b)).mpr
          fun x => by
            rw [Multiset.mem_dedup, Multiset.mem_dedup]
            exact h' x
      by_cases h1 : PairwiseM Pre a
      · rw [if_pos h1, if_pos (hp.mp h1), hd]
      · rw [if_neg h1, if_neg (fun h2 => h1 (hp.mpr h2))])
    p

/-- **The reader**: an arrival list whose pairs satisfy `Pre` folds as
the machine does (in arrival order, duplicates and all). -/
theorem foldOn_mk_coe [DecidableEq α] {σ : Type _} (Pre : α → α → Prop) [DecidableRel Pre]
    (g : σ → α → σ) (init : σ)
    (hcomm : ∀ s x y, Pre x y → g (g s x) y = g (g s y) x)
    (idem : ∀ s x, g (g s x) x = g s x) {l : List α} (hl : PairwiseL Pre l) :
    foldOn Pre g init hcomm idem (mk (↑l : Multiset α)) = l.foldl g init := by
  show (if PairwiseM Pre (↑l : Multiset α) then
    Multiset.foldlOn Pre g init hcomm (↑l : Multiset α).dedup else init) = _
  rw [if_pos (PairwiseM.coe.mpr hl), Multiset.coe_dedup,
    Multiset.foldlOn_coe Pre g init hcomm
      (PairwiseL.of_subset (List.dedup_sublist l).subset hl),
    foldl_dedup_of Pre g hcomm idem l hl init]

/-- The unconditional obligation recovers the plain multiset fold of the
arrival multiset (duplicates absorbed by idempotence). -/
theorem foldOn_true_mk [DecidableEq α] {σ : Type _} (g : σ → α → σ) (init : σ)
    (hcomm : ∀ s x y, True → g (g s x) y = g (g s y) x)
    (idem : ∀ s x, g (g s x) x = g s x) (m : Multiset α) :
    foldOn (fun _ _ => True) g init hcomm idem (mk m)
      = @Multiset.foldl α σ g ⟨fun s x y => hcomm s x y trivial⟩ init m := by
  induction m using Quot.inductionOn with
  | h l =>
    rw [show Quot.mk _ l = (↑l : Multiset α) from rfl,
      foldOn_mk_coe _ g init hcomm idem (PairwiseL.true l), Multiset.coe_foldl]

/-- Membership through the derived support view. -/
theorem mem_support_val [DecidableEq α] {x : α} {p : RetryPool α} :
    x ∈ (support p).val ↔ Mem x p := by
  induction p using Quotient.inductionOn with
  | h m =>
    show x ∈ m.dedup ↔ x ∈ m
    exact Multiset.mem_dedup

theorem mem_support [DecidableEq α] {x : α} {p : RetryPool α} : x ∈ support p ↔ x ∈ p :=
  mem_support_val

/-- Entitlement is decidable (through the support). -/
instance [DecidableEq α] (x : α) (p : RetryPool α) : Decidable (x ∈ p) :=
  decidable_of_iff (x ∈ support p) mem_support

/-- Merging entitlements (support union). -/
def union (a b : RetryPool α) : RetryPool α :=
  Quotient.lift₂ (fun m n => mk (m + n))
    (fun m n m' n' hm hn => by
      have hm' : ∀ y, y ∈ m ↔ y ∈ m' := hm
      have hn' : ∀ y, y ∈ n ↔ y ∈ n' := hn
      exact Quotient.sound fun y => by
        rw [Multiset.mem_add, Multiset.mem_add]
        exact or_congr (hm' y) (hn' y))
    a b

theorem mem_union {x : α} {a b : RetryPool α} :
    Mem x (union a b) ↔ Mem x a ∨ Mem x b := by
  induction a using Quotient.inductionOn with
  | h m =>
    induction b using Quotient.inductionOn with
    | h n =>
      show x ∈ m + n ↔ x ∈ m ∨ x ∈ n
      exact Multiset.mem_add

/-- Tracing a union fold's members back to their source pools. -/
theorem mem_foldl_union {x : α} :
    ∀ {l : List (RetryPool α)} {acc : RetryPool α},
      x ∈ l.foldl union acc → x ∈ acc ∨ ∃ p ∈ l, x ∈ p
  | [], _ => fun h => Or.inl h
  | p :: rest, acc => fun h => by
    rcases mem_foldl_union (l := rest) (acc := union acc p) h
      with h' | ⟨q, hq, hxq⟩
    · rcases mem_union.mp h' with h'' | h''
      · exact Or.inl h''
      · exact Or.inr ⟨p, List.mem_cons_self .., h''⟩
    · exact Or.inr ⟨q, List.mem_cons_of_mem _ hq, hxq⟩

/-- Growth: the entitlement's support extends. -/
def le (a b : RetryPool α) : Prop := ∀ x : α, x ∈ a → x ∈ b

theorem le_refl (a : RetryPool α) : le a a := fun _ h => h

theorem le_trans {a b c : RetryPool α} (h₁ : le a b) (h₂ : le b c) :
    le a c := fun x h => h₂ x (h₁ x h)

theorem union_le_union {a b c d : RetryPool α} (h₁ : le a c)
    (h₂ : le b d) : le (union a b) (union c d) := fun x hx => by
  rcases mem_union.mp hx with h | h
  · exact mem_union.mpr (Or.inl (h₁ x h))
  · exact mem_union.mpr (Or.inr (h₂ x h))

end RetryPool

/-! ## The graded content carrier and its growth order -/

/-- What a stream's consumers may observe, by grade. -/
@[reducible] def PoolCarrier (α : Type _) [DecidableEq α] :
    StrOrd → Retries → Type _
  | .totalOrder, .exactlyOnce => List α
  | .totalOrder, .atLeastOnce => StutterSeq α
  | .noOrder, .exactlyOnce => Multiset α
  | .noOrder, .atLeastOnce => RetryPool α

/-- `.first()` of ordered content at either retry grade: the head of the sequence, which
stutter never changes (D91). -/
@[reducible] def poolHead? {α : Type _} [DecidableEq α] :
    (ret : Retries) → PoolCarrier α .totalOrder ret → Option α
  | .exactlyOnce => List.head?
  | .atLeastOnce => StutterSeq.head?

/-- The graded fold obligation (defined here for `PoolFold`; re-exported
by the signature as `FoldOk`), **conditional on a pairwise prerequisite**
`Pre` (D71, R3): commutativity is demanded only on pairs satisfying `Pre`
(`Pre := fun _ _ => True` is the unconditional obligation); consecutive
idempotence is unconditional. Reducible: obligation proofs are passed
inline in program text, and consumers (`co_transfer` matching) must see
their types at reducible transparency. -/
@[reducible] def FoldOkP {α σ : Type _} :
    StrOrd → Retries → (Pre : α → α → Prop) → (σ → α → σ) → Prop
  | .totalOrder, .exactlyOnce => fun _ _ => True
  | .noOrder, .exactlyOnce => fun Pre g => ∀ s x y, Pre x y → g (g s x) y = g (g s y) x
  | .totalOrder, .atLeastOnce => fun _ g => ∀ s x, g (g s x) x = g s x
  | .noOrder, .atLeastOnce => fun Pre g =>
      (∀ s x y, Pre x y → g (g s x) y = g (g s y) x) ∧ (∀ s x, g (g s x) x = g s x)

/-- **The pairwise content fact** a conditional fold's coupling needs: at
the unordered grades every two members satisfy `Pre`; at the ordered
grades nothing (the machine's order IS the denotation's). This is what a
`ghost pre` proves at the denotation and what the corner's `wf` carries. -/
def PoolPairwise {α : Type _} [DecidableEq α] :
    (ord : StrOrd) → (ret : Retries) → (Pre : α → α → Prop) →
      PoolCarrier α ord ret → Prop
  | .totalOrder, _ => fun _ _ => True
  | .noOrder, .exactlyOnce => fun Pre m => PairwiseM Pre m
  | .noOrder, .atLeastOnce => fun Pre p => ∀ x ∈ p, ∀ y ∈ p, Pre x y

@[simp] theorem PoolPairwise_true {α : Type _} [DecidableEq α] (ord : StrOrd)
    (ret : Retries) (c : PoolCarrier α ord ret) :
    PoolPairwise ord ret (fun _ _ => True) c := by
  cases ord <;> cases ret <;> first | trivial | exact fun _ _ _ _ => trivial

/-- The graded growth order (Flo monotonicity's vocabulary): prefix /
prefix-mod-stutter / sub-multiset / support-inclusion. -/
def PoolLe {α : Type _} [DecidableEq α] :
    (ord : StrOrd) → (ret : Retries) →
    PoolCarrier α ord ret → PoolCarrier α ord ret → Prop
  | .totalOrder, .exactlyOnce => fun a b => a <+: b
  | .totalOrder, .atLeastOnce => StutterSeq.le
  | .noOrder, .exactlyOnce => fun a b => a ≤ b
  | .noOrder, .atLeastOnce => RetryPool.le

/-- Grade-preserving image at the exactly-once grades. -/
def mapPool {α β : Type _} [DecidableEq α] [DecidableEq β] :
    {ord : StrOrd} → (f : α → β) →
    PoolCarrier α ord .exactlyOnce → PoolCarrier β ord .exactlyOnce
  | .totalOrder => fun f l => l.map f
  | .noOrder => fun f m => m.map f

def filterMapPool {α β : Type _} [DecidableEq α] [DecidableEq β] :
    {ord : StrOrd} → (f : α → Option β) →
    PoolCarrier α ord .exactlyOnce → PoolCarrier β ord .exactlyOnce
  | .totalOrder => fun f l => l.filterMap f
  | .noOrder => fun f m => m.filterMap f

/-- The graded fold: one combinator, four grades — each unordered grade
consumes exactly its quotient's respect obligation (conditional on the
pairwise prerequisite `Pre`). This is the ONLY whole-content eliminator;
a fold with the wrong algebra does not exist. -/
def PoolFold {α σ : Type _} [DecidableEq α] :
    (ord : StrOrd) → (ret : Retries) → (Pre : α → α → Prop) → [DecidableRel Pre] →
    (g : σ → α → σ) → (init : σ) →
    (ok : @FoldOkP α σ ord ret Pre g) → PoolCarrier α ord ret → σ
  | .totalOrder, .exactlyOnce => fun _ _ g init _ l => l.foldl g init
  | .noOrder, .exactlyOnce => fun Pre _ g init comm m =>
      Multiset.foldlOn Pre g init comm m
  | .totalOrder, .atLeastOnce => fun _ _ g init idem s =>
      StutterSeq.fold g init idem s
  | .noOrder, .atLeastOnce => fun Pre _ g init ok p =>
      RetryPool.foldOn Pre g init ok.1 ok.2 p

/-- The pairwise fact is hereditary to the machine's realized content
(`ListLe`-shaped: a list below the pool, by grade). -/
theorem PoolPairwise.of_coe_le {α : Type _} [DecidableEq α] {Pre : α → α → Prop}
    {m : Multiset α} {l : List α} (h : (↑l : Multiset α) ≤ m)
    (hp : PoolPairwise .noOrder .exactlyOnce Pre m) : PairwiseL Pre l :=
  PairwiseM.coe.mp (PairwiseM.of_le h hp)

theorem PoolPairwise.of_mk_le {α : Type _} [DecidableEq α] {Pre : α → α → Prop}
    {p : RetryPool α} {l : List α} (h : RetryPool.le (.mk ↑l) p)
    (hp : PoolPairwise .noOrder .atLeastOnce Pre p) : PairwiseL Pre l :=
  fun x hx y hy => hp x (h x ((RetryPool.mem_mk x _).mpr (Multiset.mem_coe.mpr hx)))
    y (h y ((RetryPool.mem_mk y _).mpr (Multiset.mem_coe.mpr hy)))

/-- The decision vocabulary of a snapshot site, by source order: prefix
cut counts where order is kept, arrival-increment multisets where it is
gone. -/
def CutDec (α : Type _) : StrOrd → Type _
  | .totalOrder => List Nat
  | .noOrder => List (Multiset α)

instance : (ord : StrOrd) → Inhabited (CutDec α ord)
  | .totalOrder => ⟨([] : List Nat)⟩
  | .noOrder => ⟨([] : List (Multiset α))⟩

/-- The number of ticks a cut decision realizes (one cut per tick, in
either grade). -/
def CutDec.ticks {α : Type _} : {ord : StrOrd} → CutDec α ord → Nat
  | .totalOrder, d => d.length
  | .noOrder, d => d.length

/-- The least content (a cycle's starting wire). -/
def PoolBot {α : Type _} [DecidableEq α] :
    (ord : StrOrd) → (ret : Retries) → PoolCarrier α ord ret
  | .totalOrder, .exactlyOnce => []
  | .totalOrder, .atLeastOnce => StutterSeq.mk []
  | .noOrder, .exactlyOnce => 0
  | .noOrder, .atLeastOnce => RetryPool.mk 0

theorem PoolLe.refl {α : Type _} [DecidableEq α] (ord : StrOrd)
    (ret : Retries) (a : PoolCarrier α ord ret) : PoolLe ord ret a a := by
  cases ord <;> cases ret
  · exact List.prefix_refl _
  · exact StutterSeq.le_refl _
  · exact _root_.le_refl a
  · exact RetryPool.le_refl _

theorem PoolLe.trans {α : Type _} [DecidableEq α] {ord : StrOrd}
    {ret : Retries} {a b c : PoolCarrier α ord ret}
    (h₁ : PoolLe ord ret a b) (h₂ : PoolLe ord ret b c) :
    PoolLe ord ret a c := by
  cases ord <;> cases ret
  · exact List.IsPrefix.trans h₁ h₂
  · exact StutterSeq.le_trans h₁ h₂
  · exact _root_.le_trans h₁ h₂
  · exact RetryPool.le_trans h₁ h₂

/-- The least content is below everything (the Kleene seed's step). -/
theorem PoolLe.bot_le {α : Type _} [DecidableEq α] (ord : StrOrd)
    (ret : Retries) (a : PoolCarrier α ord ret) :
    PoolLe ord ret (PoolBot ord ret) a := by
  cases ord <;> cases ret
  · exact List.nil_prefix
  · exact List.nil_prefix
  · exact Multiset.zero_le a
  · exact fun x h => absurd ((RetryPool.mem_mk x 0).mp h)
      (Multiset.notMem_zero x)

/-! ## In-tick content operators on the quotient (`Values`'s
`BoundedStream` vocabulary, D60)

One tick's bounded stream content at the denotation is its grade
quotient; the in-tick operators are the quotient's own operations.
Grade-polymorphic in the order, `ExactlyOnce` only (as the stream
`map`/`filterMap`). -/

/-- `count` on one tick's exactly-once content. -/
def poolCount {α : Type _} [DecidableEq α] :
    {ord : StrOrd} → PoolCarrier α ord .exactlyOnce → Nat
  | .totalOrder => fun l => l.length
  | .noOrder => fun m => Multiset.card m

/-- `.is_empty()` at every grade (D89): no content — at the retry grades, an empty class. -/
def poolIsEmpty {α : Type _} [DecidableEq α] :
    {ord : StrOrd} → {ret : Retries} → PoolCarrier α ord ret → Bool
  | .totalOrder, .exactlyOnce => fun l => l.isEmpty
  | .noOrder, .exactlyOnce => fun m => decide (Multiset.card m = 0)
  | .totalOrder, .atLeastOnce => fun s => s.norm.isEmpty
  | .noOrder, .atLeastOnce => fun p => decide (RetryPool.support p = ∅)

/-- `destutter` erases no non-empty list. -/
theorem destutter_eq_nil_iff {α : Type _} [DecidableEq α] : ∀ (l : List α), destutter l = [] ↔ l = []
  | [] => Iff.rfl
  | [x] => by simp [destutter]
  | x :: y :: rest => by
    unfold destutter
    split
    · rw [destutter_eq_nil_iff (y :: rest)]; simp
    · simp

/-- `chain`: concatenation at `TotalOrder`, union at `NoOrder`. -/
def poolChain {α : Type _} [DecidableEq α] :
    {ord : StrOrd} → PoolCarrier α ord .exactlyOnce →
    PoolCarrier α ord .exactlyOnce → PoolCarrier α ord .exactlyOnce
  | .totalOrder => fun a b => a ++ b
  | .noOrder => fun a b => a + b

/-- `weaken_ordering`: the content, forgotten into the multiset. -/
def poolWeakenOrder {α : Type _} [DecidableEq α] :
    {ord : StrOrd} → PoolCarrier α ord .exactlyOnce → Multiset α
  | .totalOrder => fun l => (↑l : Multiset α)
  | .noOrder => fun m => m

/-- `flat_map_unordered` into the multiset: the per-element unordered
contents, summed. -/
def poolFlatMapUnordered {α β : Type _} [DecidableEq α] [DecidableEq β] :
    {ord : StrOrd} → PoolCarrier α ord .exactlyOnce → (α → Multiset β) →
    Multiset β
  | .totalOrder => fun l f => Multiset.bind (↑l) f
  | .noOrder => fun m f => Multiset.bind m f

/-- `filter_if`: this tick's content or the bottom of the grade. -/
def poolFilterIf {α : Type _} [DecidableEq α] {ord : StrOrd}
    {ret : Retries} (b : PoolCarrier α ord ret) (flag : Bool) :
    PoolCarrier α ord ret :=
  if flag then b else PoolBot ord ret

/-- `filter`: the content at the predicate (one tick, exactly-once). -/
def poolFilter {α : Type _} [DecidableEq α] :
    {ord : StrOrd} → (p : α → Bool) →
    PoolCarrier α ord .exactlyOnce → PoolCarrier α ord .exactlyOnce
  | .totalOrder => fun p l => l.filter p
  | .noOrder => fun p m => m.filter (fun x => p x = true)

/-- `.join(other)` into the multiset: every pair of entries at a common
key (both sides read as multisets; the output is `NoOrder`). -/
def listJoin {K V W : Type _} [DecidableEq K] (a : List (K × V))
    (b : List (K × W)) : List (K × (V × W)) :=
  a.flatMap (fun e => (b.filter (fun f => f.1 = e.1)).map (fun f => (e.1, (e.2, f.2))))

def multisetJoin {K V W : Type _} [DecidableEq K] (a : Multiset (K × V))
    (b : Multiset (K × W)) : Multiset (K × (V × W)) :=
  Multiset.bind a (fun e => (b.filter (fun f => f.1 = e.1)).map (fun f => (e.1, (e.2, f.2))))

theorem coe_listJoin {K V W : Type _} [DecidableEq K] (a : List (K × V))
    (b : List (K × W)) :
    (↑(listJoin a b) : Multiset (K × (V × W))) = multisetJoin (↑a) (↑b) := by
  unfold listJoin multisetJoin
  rw [← Multiset.coe_bind]
  congr 1

def poolJoin {K V W : Type _} [DecidableEq K] [DecidableEq V] [DecidableEq W]
    {ord ord' : StrOrd} (a : PoolCarrier (K × V) ord .exactlyOnce)
    (b : PoolCarrier (K × W) ord' .exactlyOnce) : Multiset (K × (V × W)) :=
  multisetJoin (poolWeakenOrder a) (poolWeakenOrder b)

/-- `.anti_join(keys)`: the entries whose key is absent from `keys`. -/
def poolAntiJoin {K V : Type _} [DecidableEq K] [DecidableEq V] {ord ord' : StrOrd}
    (a : PoolCarrier (K × V) ord .exactlyOnce)
    (ks : PoolCarrier K ord' .exactlyOnce) : PoolCarrier (K × V) ord .exactlyOnce :=
  poolFilter (fun e => !(decide (e.1 ∈ poolWeakenOrder ks))) a

/-- `.filter_not_in(other)`: the items absent from `other`. -/
def poolFilterNotIn {α : Type _} [DecidableEq α] {ord ord' : StrOrd}
    (a : PoolCarrier α ord .exactlyOnce)
    (other : PoolCarrier α ord' .exactlyOnce) : PoolCarrier α ord .exactlyOnce :=
  poolFilter (fun x => !(decide (x ∈ poolWeakenOrder other))) a

/-- `.max()`'s step: the running maximum of an ordered type. -/
def maxStep {α : Type _} [LinearOrder α] (cur : Option α) (x : α) : Option α :=
  some (match cur with | none => x | some y => Max.max y x)

theorem maxStep_comm {α : Type _} [LinearOrder α] (s : Option α) (x y : α) :
    maxStep (maxStep s x) y = maxStep (maxStep s y) x := by
  cases s with
  | none => show some (Max.max x y) = some (Max.max y x); rw [max_comm]
  | some a =>
    show some (Max.max (Max.max a x) y) = some (Max.max (Max.max a y) x)
    rw [max_right_comm]

instance {α : Type _} [LinearOrder α] :
    RightCommutative (maxStep (α := α)) :=
  ⟨fun s x y => maxStep_comm s x y⟩

/-- `.max()` on one tick's exactly-once content (`None` when empty). -/
def poolMax {α : Type _} [DecidableEq α] [LinearOrder α] :
    {ord : StrOrd} → PoolCarrier α ord .exactlyOnce → Option α
  | .totalOrder => fun l => l.foldl maxStep none
  | .noOrder => fun m => Multiset.foldl maxStep none m

/-- `.min()`'s step: the running minimum of an ordered type. -/
def minStep {α : Type _} [LinearOrder α] (cur : Option α) (x : α) : Option α :=
  some (match cur with | none => x | some y => Min.min y x)

theorem minStep_comm {α : Type _} [LinearOrder α] (s : Option α) (x y : α) :
    minStep (minStep s x) y = minStep (minStep s y) x := by
  cases s with
  | none => show some (Min.min x y) = some (Min.min y x); rw [min_comm]
  | some a =>
    show some (Min.min (Min.min a x) y) = some (Min.min (Min.min a y) x)
    rw [min_right_comm]

instance {α : Type _} [LinearOrder α] :
    RightCommutative (minStep (α := α)) :=
  ⟨fun s x y => minStep_comm s x y⟩

/-- `.min()` on one tick's exactly-once content (`None` when empty). -/
def poolMin {α : Type _} [DecidableEq α] [LinearOrder α] :
    {ord : StrOrd} → PoolCarrier α ord .exactlyOnce → Option α
  | .totalOrder => fun l => l.foldl minStep none
  | .noOrder => fun m => Multiset.foldl minStep none m

/-- `.sort()` on one tick's exactly-once content: the ascending sequence
(the runtime's merge sort; at `NoOrder` the quotient's `sort`, well
defined because a `LinearOrder` leaves no ties between distinct
elements). -/
def poolSort {α : Type _} [DecidableEq α] [LinearOrder α] :
    {ord : StrOrd} → PoolCarrier α ord .exactlyOnce → List α
  | .totalOrder => fun l => l.mergeSort (fun a b => decide (a ≤ b))
  | .noOrder => fun m => Multiset.sort m (· ≤ ·)

/-- `.cross_singleton(optional)`: nothing while the optional is `None`. -/
def poolCrossOptional {α σ : Type _} [DecidableEq α] [DecidableEq σ] :
    {ord : StrOrd} → PoolCarrier α ord .exactlyOnce → Option σ →
      PoolCarrier (α × σ) ord .exactlyOnce
  | ord, b, some s => mapPool (ord := ord) (fun a => (a, s)) b
  | ord, _, none => PoolBot ord .exactlyOnce

/-- `enumerate` on ordered content: `(index, item)` pairs. -/
def listEnumerate {α : Type _} (l : List α) : List (Nat × α) :=
  l.zipIdx.map (fun p => (p.2, p.1))

/-- The indices of an enumeration, offset by a base, are the next
`l.length` naturals from the base (`List.range'`): consecutive, no gaps,
no repeats. -/
theorem listEnumerate_indices_add {α : Type _} (base : Nat) (l : List α) :
    (listEnumerate l).map (fun p => base + p.1) = List.range' base l.length := by
  unfold listEnumerate
  rw [List.map_map]
  suffices h : ∀ (l : List α) (k : Nat),
      (l.zipIdx k).map (fun p => base + p.2) = List.range' (base + k) l.length by
    have := h l 0
    rw [Nat.add_zero] at this
    exact this
  intro l
  induction l with
  | nil => intro k; rfl
  | cons p rest ih =>
    intro k
    rw [List.zipIdx_cons, List.map_cons, ih (k + 1), List.length_cons, List.range'_succ,
      ← Nat.add_assoc]

/-! ## The pool operators at concrete grades — the `den` simp set

Each pool operator is a `match` on the grade; at a concrete grade it IS
the `List`/`Multiset` operation named by the Rust method. Stated as
rewrite rules so a `tick` block's generated step — `H.bmap` ↦ `mapPool`
↦ `List.map` — reads as the plain term of the Rust line under `simp only
[<out>_step, den]` (FINDINGS D64, E6: no hand-written pure twin of
a body, no body-shape bridge). -/
section TickBody
variable {α β : Type _} [DecidableEq α] [DecidableEq β]

@[den] theorem mapPool_totalOrder (f : α → β) (l : List α) :
    mapPool (ord := .totalOrder) f l = l.map f := rfl
@[den] theorem mapPool_noOrder (f : α → β) (m : Multiset α) :
    mapPool (ord := .noOrder) f m = m.map f := rfl
@[den] theorem filterMapPool_totalOrder (f : α → Option β) (l : List α) :
    filterMapPool (ord := .totalOrder) f l = l.filterMap f := rfl
@[den] theorem filterMapPool_noOrder (f : α → Option β) (m : Multiset α) :
    filterMapPool (ord := .noOrder) f m = m.filterMap f := rfl
@[den] theorem poolCount_totalOrder (l : List α) :
    poolCount (ord := .totalOrder) l = l.length := rfl
@[den] theorem poolCount_noOrder (m : Multiset α) :
    poolCount (ord := .noOrder) m = Multiset.card m := rfl
@[den] theorem poolChain_totalOrder (a b : List α) :
    poolChain (ord := .totalOrder) a b = a ++ b := rfl
@[den] theorem poolChain_noOrder (a b : Multiset α) :
    poolChain (ord := .noOrder) a b = a + b := rfl
@[den] theorem poolWeakenOrder_totalOrder (l : List α) :
    poolWeakenOrder (ord := .totalOrder) l = (↑l : Multiset α) := rfl
@[den] theorem poolWeakenOrder_noOrder (m : Multiset α) :
    poolWeakenOrder (ord := .noOrder) m = m := rfl
@[den] theorem poolFlatMapUnordered_totalOrder (l : List α) (f : α → Multiset β) :
    poolFlatMapUnordered (ord := .totalOrder) l f = Multiset.bind (↑l) f := rfl
@[den] theorem poolFlatMapUnordered_noOrder (m : Multiset α) (f : α → Multiset β) :
    poolFlatMapUnordered (ord := .noOrder) m f = Multiset.bind m f := rfl
@[den] theorem poolFilter_totalOrder (p : α → Bool) (l : List α) :
    poolFilter (ord := .totalOrder) p l = l.filter p := rfl
@[den] theorem poolFilter_noOrder (p : α → Bool) (m : Multiset α) :
    poolFilter (ord := .noOrder) p m = m.filter (fun x => p x = true) := rfl
@[den] theorem poolFilterIf_true {ord : StrOrd} {ret : Retries}
    (b : PoolCarrier α ord ret) : poolFilterIf b true = b := rfl
@[den] theorem poolFilterIf_false {ord : StrOrd} {ret : Retries}
    (b : PoolCarrier α ord ret) : poolFilterIf b false = PoolBot ord ret := rfl
@[den] theorem PoolBot_totalOrder_exactlyOnce :
    (PoolBot .totalOrder .exactlyOnce : PoolCarrier α .totalOrder .exactlyOnce) = [] := rfl
@[den] theorem PoolBot_noOrder_exactlyOnce :
    (PoolBot .noOrder .exactlyOnce : PoolCarrier α .noOrder .exactlyOnce) = 0 := rfl
@[den] theorem PoolFold_totalOrder_exactlyOnce {σ : Type _} (Pre : α → α → Prop)
    [DecidableRel Pre] (g : σ → α → σ) (init : σ)
    (ok : FoldOkP .totalOrder .exactlyOnce Pre g) (l : List α) :
    PoolFold .totalOrder .exactlyOnce Pre g init ok l = l.foldl g init := rfl
@[den] theorem PoolFold_noOrder_exactlyOnce {σ : Type _} (Pre : α → α → Prop)
    [DecidableRel Pre] (g : σ → α → σ) (init : σ)
    (ok : FoldOkP .noOrder .exactlyOnce Pre g) (m : Multiset α) :
    PoolFold .noOrder .exactlyOnce Pre g init ok m = Multiset.foldlOn Pre g init ok m := rfl
attribute [den] Multiset.foldlOn_true
@[den] theorem poolMax_totalOrder [LinearOrder α] (l : List α) :
    poolMax (ord := .totalOrder) l = l.foldl maxStep none := rfl
@[den] theorem poolMax_noOrder [LinearOrder α] (m : Multiset α) :
    poolMax (ord := .noOrder) m = Multiset.foldl maxStep none m := rfl
attribute [den] poolJoin poolAntiJoin poolFilterNotIn listEnumerate
end TickBody

/-! ## Per-key pool algebra (generic)

The facts a keyed `tick` body's obligations need about the quotient
operations once read under `den`: an `anti_join`/`filter_not_in` is a
plain filter on the complement (stated with the membership's `Decidable`
instance as an explicit argument — after `den` the instance is over the
unfolded `poolWeakenOrder` form while the proposition reads the pool,
FINDINGS D64 gotcha v), a key-predicate filter restricted to one key is
all-or-nothing, and the count of a key in a filtered dedup of a key
projection is the indicator. -/
section KeyedAlgebra
variable {K α : Type _} [DecidableEq K]

/-- A key-predicate filter restricted to one key is all-or-nothing. -/
theorem filter_key_of_pred (m : Multiset (K × α))
    (P : K → Prop) [DecidablePred P] (k : K) :
    (m.filter (fun r => P r.1)).filter (fun r => r.1 = k)
      = if P k then m.filter (fun r => r.1 = k) else 0 := by
  by_cases hp : P k
  · rw [if_pos hp, Multiset.filter_filter]
    exact Multiset.filter_congr (fun r _ => by
      constructor
      · rintro ⟨h1, -⟩
        exact h1
      · intro h1
        exact ⟨h1, by rw [h1]; exact hp⟩)
  · rw [if_neg hp, Multiset.filter_filter, Multiset.filter_eq_nil.mpr]
    rintro r - ⟨h1, h2⟩
    exact hp (h1 ▸ h2)

/-- Count of a key in a filtered dedup of the key projection. -/
theorem count_dedup_keys_filter (m : Multiset (K × α))
    (P : K → Prop) [DecidablePred P] (k : K) :
    (((m.map Prod.fst).dedup).filter P).count k
      = if k ∈ m.map Prod.fst ∧ P k then 1 else 0 := by
  rw [Multiset.count_filter]
  by_cases hp : P k
  · rw [if_pos hp, Multiset.count_dedup]
    by_cases hm : k ∈ m.map Prod.fst
    · rw [if_pos hm, if_pos ⟨hm, hp⟩]
    · rw [if_neg hm, if_neg (fun hc => hm hc.1)]
  · rw [if_neg hp, if_neg (fun hc => hp hc.2)]

/-- Count of a key in a doubly filtered dedup of the key projection. -/
theorem count_dedup_keys_filter₂ (m : Multiset (K × α))
    (P Q : K → Prop) [DecidablePred P] [DecidablePred Q] (k : K) :
    ((((m.map Prod.fst).dedup).filter P).filter Q).count k
      = if k ∈ m.map Prod.fst ∧ P k ∧ Q k then 1 else 0 := by
  rw [Multiset.count_filter, count_dedup_keys_filter]
  by_cases hq : Q k
  · by_cases hpm : k ∈ m.map Prod.fst ∧ P k
    · rw [if_pos hpm, if_pos hq, if_pos ⟨hpm.1, hpm.2, hq⟩]
    · rw [if_neg hpm, if_pos hq, if_neg (fun hc => hpm ⟨hc.1, hc.2.1⟩)]
  · rw [if_neg hq, if_neg (fun hc => hq hc.2.2)]

/-- A key of the pool is a key of its dedup'd key list. -/
theorem mem_keys_of_mem {m : Multiset (K × α)} {r : K × α}
    (hr : r ∈ m) : r.1 ∈ (m.map Prod.fst).dedup :=
  Multiset.mem_dedup.mpr (Multiset.mem_map.mpr ⟨r, hr, rfl⟩)

/-! ### Per-key arrivals (`keyVals`: the values a key received, in
arrival order — the reader of ordered keyed content) -/

/-- The values that arrived at key `k`, in arrival order. -/
def keyVals (k : K) (xs : List (K × α)) : List α :=
  (xs.filter (fun e => decide (e.1 = k))).map Prod.snd

theorem keyVals_append (k : K) (xs ys : List (K × α)) :
    keyVals k (xs ++ ys) = keyVals k xs ++ keyVals k ys := by
  simp only [keyVals, List.filter_append, List.map_append]

theorem keyVals_singleton (k : K) (e : K × α) :
    keyVals k [e] = if e.1 = k then [e.2] else [] := by
  unfold keyVals
  by_cases h : e.1 = k
  · simp [h]
  · simp [h]

/-- A value at `k` is an entry `(k, v)`. -/
theorem mem_keyVals {k : K} {xs : List (K × α)} {v : α} (h : v ∈ keyVals k xs) :
    (k, v) ∈ xs := by
  unfold keyVals at h
  obtain ⟨⟨k₁, v₁⟩, hmem, rfl⟩ := List.mem_map.mp h
  obtain ⟨hmem, hk⟩ := List.mem_filter.mp hmem
  rw [show k₁ = k from of_decide_eq_true hk] at hmem
  exact hmem

/-- Re-keying the values gives back the key's entries, as a sublist of
the source. -/
theorem keyVals_map_pair_sublist (k : K) (xs : List (K × α)) :
    List.Sublist ((keyVals k xs).map (fun v => (k, v))) xs := by
  unfold keyVals
  rw [List.map_map]
  have : (xs.filter (fun e => decide (e.1 = k))).map ((fun v => (k, v)) ∘ Prod.snd)
      = xs.filter (fun e => decide (e.1 = k)) := by
    conv_rhs => rw [← List.map_id (xs.filter _)]
    refine List.map_congr_left (fun e he => ?_)
    obtain ⟨-, hk⟩ := List.mem_filter.mp he
    obtain ⟨e1, e2⟩ := e
    simp only [Function.comp, id]
    rw [show e1 = k from of_decide_eq_true hk]
  rw [this]
  exact List.filter_sublist

theorem keyVals_prefix {k : K} {xs ys : List (K × α)} (h : xs <+: ys) :
    keyVals k xs <+: keyVals k ys := by
  obtain ⟨e, rfl⟩ := h
  rw [keyVals_append]
  exact List.prefix_append _ _

end KeyedAlgebra

end Hydro
