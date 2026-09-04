import Mathlib.Data.Multiset.Basic
import Mathlib.Data.Multiset.AddSub
import Mathlib.Data.Multiset.MapFold
import Mathlib.Data.Multiset.Dedup
import Mathlib.Data.Multiset.Bind
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
by the signature as `FoldOk`). Reducible: obligation proofs are passed
inline in program text, and consumers (`co_transfer` matching) must see
their types at reducible transparency. -/
@[reducible] def FoldOkP {α σ : Type _} :
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

instance : (ord : StrOrd) → Inhabited (CutDec α ord)
  | .totalOrder => ⟨([] : List Nat)⟩
  | .noOrder => ⟨([] : List (Multiset α))⟩

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

/-! ### The keyed vocabulary, collapsed onto entry streams

Rust's `KeyedStream`/`KeyedSingleton` are read back as their
`entries()`: a `NoOrder` stream of `(key, value)` pairs with each key
once (the keyed fold's result). A dedicated keyed carrier is deferred;
these are the operators quorum.rs / request_response.rs use on them. -/

/-- `into_keyed().fold(init, g).entries()` on a plain list: one entry
per distinct key, the key's values folded in arrival order. -/
def keyedFoldList {K V A : Type _} [DecidableEq K] (g : A → V → A) (init : A)
    (l : List (K × V)) : List (K × A) :=
  (l.map Prod.fst).dedup.map
    (fun k => (k, ((l.filter (fun e => e.1 = k)).map Prod.snd).foldl g init))

/-- The same on a multiset: the per-key fold needs the commutative
algebra (quorum.rs's `commutative = manual_proof!`). -/
def keyedFoldMultiset {K V A : Type _} [DecidableEq K] (g : A → V → A) (init : A)
    (comm : ∀ s x y, g (g s x) y = g (g s y) x) (m : Multiset (K × V)) :
    Multiset (K × A) :=
  (m.map Prod.fst).dedup.map
    (fun k => (k, @Multiset.foldl V A g ⟨fun s x y => comm s x y⟩ init
      ((m.filter (fun e => e.1 = k)).map Prod.snd)))

/-- The multiset keyed fold of a list is the list keyed fold. -/
theorem coe_keyedFoldList {K V A : Type _} [DecidableEq K] (g : A → V → A) (init : A)
    (comm : ∀ s x y, g (g s x) y = g (g s y) x) (l : List (K × V)) :
    (↑(keyedFoldList g init l) : Multiset (K × A))
      = keyedFoldMultiset g init comm (↑l : Multiset (K × V)) := by
  unfold keyedFoldList keyedFoldMultiset
  rw [← Multiset.map_coe, ← Multiset.coe_dedup, ← Multiset.map_coe]
  congr 1

/-- The graded keyed fold: ordered content folds in arrival order (no
algebra needed), unordered content through the commutative fold. -/
def poolKeyedFold {K V A : Type _} [DecidableEq K] [DecidableEq V] :
    {ord : StrOrd} → (g : A → V → A) → (init : A) →
    FoldOkP ord .exactlyOnce g →
    PoolCarrier (K × V) ord .exactlyOnce → Multiset (K × A)
  | .totalOrder => fun g init _ l => (↑(keyedFoldList g init l) : Multiset (K × A))
  | .noOrder => fun g init comm m => keyedFoldMultiset g init comm m

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
@[den] theorem PoolFold_totalOrder_exactlyOnce {σ : Type _} (g : σ → α → σ) (init : σ)
    (ok : FoldOkP .totalOrder .exactlyOnce g) (l : List α) :
    PoolFold .totalOrder .exactlyOnce g init ok l = l.foldl g init := rfl
@[den] theorem PoolFold_noOrder_exactlyOnce {σ : Type _} (g : σ → α → σ) (init : σ)
    (ok : FoldOkP .noOrder .exactlyOnce g) (m : Multiset α) :
    PoolFold .noOrder .exactlyOnce g init ok m
      = @Multiset.foldl α σ g ⟨fun s x y => ok s x y⟩ init m := rfl
@[den] theorem poolMax_totalOrder [LinearOrder α] (l : List α) :
    poolMax (ord := .totalOrder) l = l.foldl maxStep none := rfl
@[den] theorem poolMax_noOrder [LinearOrder α] (m : Multiset α) :
    poolMax (ord := .noOrder) m = Multiset.foldl maxStep none m := rfl
@[den] theorem poolKeyedFold_totalOrder {K V A : Type _} [DecidableEq K] [DecidableEq V]
    (g : A → V → A) (init : A) (ok : FoldOkP .totalOrder .exactlyOnce g)
    (l : List (K × V)) :
    poolKeyedFold (ord := .totalOrder) g init ok l = (↑(keyedFoldList g init l) : Multiset (K × A)) := rfl
@[den] theorem poolKeyedFold_noOrder {K V A : Type _} [DecidableEq K] [DecidableEq V]
    (g : A → V → A) (init : A) (ok : FoldOkP .noOrder .exactlyOnce g)
    (m : Multiset (K × V)) :
    poolKeyedFold (ord := .noOrder) g init ok m = keyedFoldMultiset g init ok m := rfl
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

/-! ### The capped keyed fold (`KeyedStream::fold_early_stop` with a
length stop, as an association list of buckets)

Inserting `(k, v)` appends `v` to `k`'s bucket while it holds fewer than
`cap` values (a fresh key opens a singleton bucket); a full bucket is
frozen. The whole fold has a CLOSED FORM: the bucket at `k` is the first
`max cap 1` values that arrived at `k`, in order — every bucket fact
(source, cap, freeze, multiplicity, uniqueness) is a `List.take`/`filter`
fact. -/

/-- One `fold_early_stop` insertion into the keyed buckets. -/
def insertCapped (cap : Nat) : List (K × List α) → K → α → List (K × List α)
  | [], k, v => [(k, [v])]
  | (k', vs) :: rest, k, v =>
    if k' = k then
      if vs.length < cap then (k', vs ++ [v]) :: rest else (k', vs) :: rest
    else (k', vs) :: insertCapped cap rest k v

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

/-- The keys after an insertion: the old keys, plus `k` if fresh. -/
theorem insertCapped_keys (cap : Nat) (bs : List (K × List α)) (k : K) (v : α) :
    (insertCapped cap bs k v).map Prod.fst
      = if k ∈ bs.map Prod.fst then bs.map Prod.fst else bs.map Prod.fst ++ [k] := by
  induction bs with
  | nil => simp [insertCapped]
  | cons hd rest ih =>
    obtain ⟨k', vs⟩ := hd
    simp only [insertCapped, List.map_cons, List.mem_cons]
    by_cases hk : k' = k
    · subst hk
      by_cases hlen : vs.length < cap
      · simp [hlen]
      · simp [hlen]
    · rw [if_neg hk, List.map_cons, ih]
      by_cases hmem : k ∈ rest.map Prod.fst
      · rw [if_pos hmem, if_pos (Or.inr hmem)]
      · rw [if_neg hmem, if_neg (fun h => h.elim (fun h => hk h.symm) hmem), List.cons_append]

/-- Membership after an insertion into nodup-keyed buckets: a foreign key's
bucket is untouched; `k`'s bucket is appended (if not full) or frozen, or
opened as `[v]` if `k` was fresh. -/
theorem mem_insertCapped {cap : Nat} {bs : List (K × List α)}
    (hnd : (bs.map Prod.fst).Nodup) (k : K) (v : α) (k' : K) (vs' : List α) :
    (k', vs') ∈ insertCapped cap bs k v ↔
      if k' = k then
        (∃ vs, (k, vs) ∈ bs ∧ vs' = (if vs.length < cap then vs ++ [v] else vs))
          ∨ (k ∉ bs.map Prod.fst ∧ vs' = [v])
      else (k', vs') ∈ bs := by
  induction bs with
  | nil =>
    simp only [insertCapped, List.mem_singleton, Prod.mk.injEq, List.not_mem_nil,
      false_and, exists_const, List.map_nil, not_false_eq_true, true_and, false_or]
    by_cases h : k' = k
    · subst h; simp
    · simp [h]
  | cons hd rest ih =>
    obtain ⟨k₀, vs₀⟩ := hd
    rw [List.map_cons, List.nodup_cons] at hnd
    have ih' := ih hnd.2
    simp only [insertCapped]
    by_cases hk₀ : k₀ = k
    · subst hk₀
      rw [if_pos rfl]
      -- the head IS `k`'s bucket; `k` is absent from the rest
      have hrest : ∀ vs, (k₀, vs) ∉ rest := fun vs h =>
        hnd.1 (List.mem_map.mpr ⟨(k₀, vs), h, rfl⟩)
      by_cases hk' : k' = k₀
      · subst hk'
        rw [if_pos rfl]
        constructor
        · intro h
          refine Or.inl ⟨vs₀, List.mem_cons_self .., ?_⟩
          by_cases hlen : vs₀.length < cap
          · rw [if_pos hlen] at h ⊢
            rcases List.mem_cons.mp h with h | h
            · exact (Prod.mk.inj h).2
            · exact absurd h (hrest _)
          · rw [if_neg hlen] at h ⊢
            rcases List.mem_cons.mp h with h | h
            · exact (Prod.mk.inj h).2
            · exact absurd h (hrest _)
        · rintro (⟨vs, hvs, rfl⟩ | ⟨habs, -⟩)
          · rcases List.mem_cons.mp hvs with h | h
            · obtain rfl := (Prod.mk.inj h).2
              by_cases hlen : vs.length < cap
              · rw [if_pos hlen, if_pos hlen]; exact List.mem_cons_self ..
              · rw [if_neg hlen, if_neg hlen]; exact List.mem_cons_self ..
            · exact absurd h (hrest _)
          · exact absurd (List.mem_cons_self ..) habs
      · rw [if_neg hk']
        have hne : ∀ l, ((k', vs') : K × List α) ≠ (k₀, l) :=
          fun l h => hk' (Prod.mk.inj h).1
        by_cases hlen : vs₀.length < cap
        · rw [if_pos hlen, List.mem_cons, List.mem_cons]
          exact or_congr_left ⟨fun h => absurd h (hne _), fun h => absurd h (hne _)⟩
        · rw [if_neg hlen]
    · rw [if_neg hk₀, List.mem_cons, ih']
      have hne : ((k', vs') : K × List α) = (k₀, vs₀) → k' = k₀ :=
        fun h => (Prod.mk.inj h).1
      by_cases hk' : k' = k
      · subst hk'
        rw [if_pos rfl, if_pos rfl]
        have hne' : ((k', vs') : K × List α) ≠ (k₀, vs₀) :=
          fun h => hk₀ (hne h).symm
        rw [List.map_cons, List.mem_cons]
        constructor
        · rintro (h | ⟨vs, hvs, h⟩ | ⟨habs, h⟩)
          · exact absurd h hne'
          · exact Or.inl ⟨vs, List.mem_cons_of_mem _ hvs, h⟩
          · exact Or.inr ⟨fun h => h.elim (fun h => hk₀ h.symm) habs, h⟩
        · rintro (⟨vs, hvs, h⟩ | ⟨habs, h⟩)
          · rcases List.mem_cons.mp hvs with hvs | hvs
            · exact absurd (Prod.mk.inj hvs).1 (fun h => hk₀ h.symm)
            · exact Or.inr (Or.inl ⟨vs, hvs, h⟩)
          · exact Or.inr (Or.inr ⟨fun h => habs (Or.inr h), h⟩)
      · rw [if_neg hk', if_neg hk', List.mem_cons]

/-- Appending one value to a nonempty capped prefix: the bucket's own
`fold_early_stop` test (`length < cap`) decides whether it grows. -/
theorem take_maxcap_snoc {cap : Nat} {l : List α} (hl : l ≠ []) (v : α) :
    (l ++ [v]).take (max cap 1)
      = if (l.take (max cap 1)).length < cap then l.take (max cap 1) ++ [v]
        else l.take (max cap 1) := by
  have hlen : 1 ≤ l.length := List.length_pos_of_ne_nil hl
  rw [List.length_take]
  by_cases h : l.length < cap
  · have hc : max cap 1 = cap := Nat.max_eq_left (by omega)
    rw [hc, if_pos (by omega), List.take_of_length_le (Nat.le_of_lt h),
      List.take_of_length_le (by rw [List.length_append, List.length_singleton]; omega)]
  · rw [if_neg (by omega), List.take_append_of_le_length (by omega)]

/-- One insertion keeps the closed form, for the source extended by `e`. -/
theorem insertCapped_spec {cap : Nat} {pfx : List (K × α)} {bs : List (K × List α)}
    (hnd : (bs.map Prod.fst).Nodup)
    (hmem : ∀ k vs, (k, vs) ∈ bs ↔
      keyVals k pfx ≠ [] ∧ vs = (keyVals k pfx).take (max cap 1))
    (e : K × α) :
    ((insertCapped cap bs e.1 e.2).map Prod.fst).Nodup ∧
    ∀ k vs, (k, vs) ∈ insertCapped cap bs e.1 e.2 ↔
      keyVals k (pfx ++ [e]) ≠ [] ∧ vs = (keyVals k (pfx ++ [e])).take (max cap 1) := by
  -- `k ∈ keys bs ↔ k arrived`
  have hkeys : ∀ k, k ∈ bs.map Prod.fst ↔ keyVals k pfx ≠ [] := by
    intro k
    constructor
    · intro h
      obtain ⟨⟨k₁, vs₁⟩, hb, rfl⟩ := List.mem_map.mp h
      exact ((hmem _ _).mp hb).1
    · intro h
      exact List.mem_map.mpr
        ⟨(k, (keyVals k pfx).take (max cap 1)), (hmem _ _).mpr ⟨h, rfl⟩, rfl⟩
  refine ⟨?_, ?_⟩
  · rw [insertCapped_keys]
    split
    · exact hnd
    · exact List.Nodup.append hnd (List.nodup_singleton _)
        (List.disjoint_singleton.mpr ‹e.1 ∉ bs.map Prod.fst›)
  · intro k vs
    rw [mem_insertCapped hnd, keyVals_append, keyVals_singleton]
    by_cases hk : k = e.1
    · rw [if_pos hk, if_pos hk.symm]
      rw [hk]
      constructor
      · rintro (⟨vs₀, hvs₀, rfl⟩ | ⟨hfresh, rfl⟩)
        · obtain ⟨hne, rfl⟩ := (hmem _ _).mp hvs₀
          exact ⟨by simp, (take_maxcap_snoc hne e.2).symm⟩
        · rw [hkeys] at hfresh
          push Not at hfresh
          rw [hfresh]
          simp
      · rintro ⟨-, rfl⟩
        by_cases hfresh : keyVals e.1 pfx = []
        · refine Or.inr ⟨fun h => (hkeys _).mp h hfresh, ?_⟩
          rw [hfresh]; simp
        · exact Or.inl ⟨(keyVals e.1 pfx).take (max cap 1), (hmem _ _).mpr ⟨hfresh, rfl⟩,
            take_maxcap_snoc hfresh e.2⟩
    · rw [if_neg hk, if_neg (fun h => hk h.symm), List.append_nil]
      exact hmem k vs

/-- **The closed form of the capped keyed fold**: the buckets of
`fold_early_stop` over `xs` have duplicate-free keys, and `(k, vs)` is a
bucket exactly when `k` arrived and `vs` is the first `max cap 1` values
at `k` — in arrival order. -/
theorem foldl_insertCapped_spec (cap : Nat) (xs : List (K × α)) :
    ((xs.foldl (fun acc e => insertCapped cap acc e.1 e.2) []).map Prod.fst).Nodup ∧
    ∀ (k : K) (vs : List α),
      (k, vs) ∈ xs.foldl (fun acc e => insertCapped cap acc e.1 e.2) [] ↔
        keyVals k xs ≠ [] ∧ vs = (keyVals k xs).take (max cap 1) := by
  suffices hgen : ∀ (ys pfx : List (K × α)) (bs : List (K × List α)),
      (bs.map Prod.fst).Nodup →
      (∀ k vs, (k, vs) ∈ bs ↔ keyVals k pfx ≠ [] ∧ vs = (keyVals k pfx).take (max cap 1)) →
      ((ys.foldl (fun acc e => insertCapped cap acc e.1 e.2) bs).map Prod.fst).Nodup ∧
      ∀ k vs, (k, vs) ∈ ys.foldl (fun acc e => insertCapped cap acc e.1 e.2) bs ↔
        keyVals k (pfx ++ ys) ≠ [] ∧ vs = (keyVals k (pfx ++ ys)).take (max cap 1) by
    have := hgen xs [] [] List.nodup_nil (fun k vs => by simp [keyVals])
    simpa using this
  intro ys
  induction ys with
  | nil =>
    intro pfx bs hnd hmem
    simpa using ⟨hnd, hmem⟩
  | cons e rest ih =>
    intro pfx bs hnd hmem
    rw [List.foldl_cons]
    obtain ⟨hnd', hmem'⟩ := insertCapped_spec hnd hmem e
    have := ih (pfx ++ [e]) _ hnd' hmem'
    simpa [List.append_assoc] using this

/-- **Full buckets freeze**: once `max cap 1` values arrived at `k`, every
later prefix's bucket at `k` is the same list. -/
theorem keyVals_take_of_prefix {c : Nat} {k : K} {xs ys : List (K × α)}
    (h : xs <+: ys) (hfull : c ≤ (keyVals k xs).length) :
    (keyVals k ys).take c = (keyVals k xs).take c := by
  obtain ⟨e, he⟩ := keyVals_prefix (k := k) h
  rw [← he, List.take_append_of_le_length hfull]

end KeyedAlgebra

end Hydro
