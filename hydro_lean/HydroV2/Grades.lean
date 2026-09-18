import Mathlib.Data.Multiset.Basic
import Mathlib.Data.Multiset.AddSub
import Mathlib.Data.Multiset.MapFold
import Mathlib.Data.Multiset.Dedup
import Mathlib.Data.Finset.Basic
import HydroV2.Order

/-!
# HydroV2 · graded stream content (the anti-cheating core)

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

namespace HydroV2

/-- Rust's stream ordering marker. -/
inductive StrOrd where
  | totalOrder
  | noOrder
deriving DecidableEq

/-- Rust's stream retries marker (`hydro_lang` `Retries`). -/
inductive Retries where
  | exactlyOnce
  | atLeastOnce
deriving DecidableEq

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

end StutterSeq

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

/-- Commutative + idempotent steps ignore retry multiplicity: folding an
execution equals folding its deduplication. -/
theorem foldl_dedup [DecidableEq α] {σ : Type _} (g : σ → α → σ)
    (comm : ∀ s x y, g (g s x) y = g (g s y) x)
    (idem : ∀ s x, g (g s x) x = g s x) (m : Multiset α) (init : σ) :
    @Multiset.foldl α σ g ⟨fun s x y => (comm s x y)⟩ init m
      = @Multiset.foldl α σ g ⟨fun s x y => (comm s x y)⟩ init m.dedup := by
  induction m using Multiset.induction_on generalizing init with
  | empty => rfl
  | cons x s ih =>
    by_cases hx : x ∈ s
    · rw [Multiset.dedup_cons_of_mem hx, ← ih init]
      -- absorbing `x` first is a no-op: `x` recurs in `s`; pull its
      -- other copy adjacent (commutativity) and collapse (idempotence).
      obtain ⟨t, rfl⟩ := Multiset.exists_cons_of_mem hx
      simp only [Multiset.foldl_cons, idem]
    · rw [Multiset.dedup_cons_of_notMem hx]
      simp only [Multiset.foldl_cons]
      exact ih (g init x)

/-- **The consumer gate**: a fold exists only for commutative *and*
idempotent steps — support-equal executions must fold equal. -/
def fold [DecidableEq α] {σ : Type _} (g : σ → α → σ) (init : σ)
    (comm : ∀ s x y, g (g s x) y = g (g s y) x)
    (idem : ∀ s x, g (g s x) x = g s x) (p : RetryPool α) : σ :=
  Quotient.lift
    (fun m => @Multiset.foldl α σ g ⟨fun s x y => (comm s x y)⟩ init m)
    (fun a b h => by
      rw [foldl_dedup g comm idem a init, foldl_dedup g comm idem b init]
      have h' : ∀ y, y ∈ a ↔ y ∈ b := h
      have hd : a.dedup = b.dedup :=
        ((Multiset.nodup_dedup a).ext (Multiset.nodup_dedup b)).mpr
          fun x => by
            rw [Multiset.mem_dedup, Multiset.mem_dedup]
            exact h' x
      rw [hd])
    p

@[simp] theorem fold_mk [DecidableEq α] {σ : Type _} (g : σ → α → σ)
    (init : σ) (comm : ∀ s x y, g (g s x) y = g (g s y) x)
    (idem : ∀ s x, g (g s x) x = g s x) (m : Multiset α) :
    fold g init comm idem (mk m)
      = @Multiset.foldl α σ g ⟨fun s x y => (comm s x y)⟩ init m := rfl

/-- Membership through the derived support view. -/
theorem mem_support_val [DecidableEq α] {x : α} {p : RetryPool α} :
    x ∈ (support p).val ↔ Mem x p := by
  induction p using Quotient.inductionOn with
  | h m =>
    show x ∈ m.dedup ↔ x ∈ m
    exact Multiset.mem_dedup

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

/-- The graded fold obligation (defined here for `PoolFold`; re-exported
by the signature as `FoldOk`). -/
def FoldOkP {α σ : Type _} :
    StrOrd → Retries → (σ → α → σ) → Prop
  | .totalOrder, .exactlyOnce => fun _ => True
  | .noOrder, .exactlyOnce => fun g => ∀ s x y, g (g s x) y = g (g s y) x
  | .totalOrder, .atLeastOnce => fun g => ∀ s x, g (g s x) x = g s x
  | .noOrder, .atLeastOnce => fun g =>
      (∀ s x y, g (g s x) y = g (g s y) x) ∧ (∀ s x, g (g s x) x = g s x)

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
consumes exactly its quotient's respect obligation. This is the ONLY
whole-content eliminator; a fold with the wrong algebra does not exist. -/
def PoolFold {α σ : Type _} [DecidableEq α] :
    (ord : StrOrd) → (ret : Retries) → (g : σ → α → σ) → (init : σ) →
    (ok : @FoldOkP α σ ord ret g) → PoolCarrier α ord ret → σ
  | .totalOrder, .exactlyOnce => fun g init _ l => l.foldl g init
  | .noOrder, .exactlyOnce => fun g init comm m =>
      @Multiset.foldl α σ g ⟨fun s x y => comm s x y⟩ init m
  | .totalOrder, .atLeastOnce => fun g init idem s =>
      StutterSeq.fold g init idem s
  | .noOrder, .atLeastOnce => fun g init ok p =>
      RetryPool.fold g init ok.1 ok.2 p

/-- The decision vocabulary of a snapshot site, by source order: prefix
cut counts where order is kept, arrival-increment multisets where it is
gone. -/
def CutDec (α : Type _) : StrOrd → Type _
  | .totalOrder => List Nat
  | .noOrder => List (Multiset α)

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

end HydroV2
