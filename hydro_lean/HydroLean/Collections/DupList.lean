import HydroLean.Prelude

/-!
# `[T]dup`: sequences modulo adjacent duplicates

(Metatheory substrate for Flo/Gyatso — the algebra/simp API here is kept
complete whether or not each lemma is currently consumed.)

The dissertation's collection type for at-least-once delivery (§3.5.2,
Fig 3.8): a retrying network may deliver an element several times *in a row*,
so the collection identifies streams that differ only in consecutive
duplicates. Downstream logic that is congruent for this identification (e.g.
`fold_idempotent`, Fig 3.11) never observes whether a retry happened — the
nondeterminism of the retry schedule is quotiented away.

We use the canonical-form presentation rather than a quotient: values are lists
with no two adjacent equal elements (`AdjNodup`), and concatenation `cat`
collapses the seam. Because both operands are canonical, at most one element
collapses at the seam (Fig 3.8's middle rule). Crucially, `cat` is associative
on *all* lists (`cat_assoc`) — this is the algebraic fact behind the eager
execution proof for `network_o2o_retry` (§3.5.2: "concatenation is associative
down to individual elements").

Lists here are oldest-first (standard append order); the dissertation renders
streams newest-first, which is just the mirror image.
-/

namespace HydroLean

universe u

variable {α : Type u}

/-- No two adjacent elements are equal. -/
def AdjNodup : List α → Prop
  | [] => True
  | [_] => True
  | a :: b :: l => a ≠ b ∧ AdjNodup (b :: l)

@[simp] theorem adjNodup_nil : AdjNodup ([] : List α) := trivial
@[simp] theorem adjNodup_single (a : α) : AdjNodup [a] := trivial
theorem adjNodup_cons_cons {a b : α} {l : List α} :
    AdjNodup (a :: b :: l) ↔ a ≠ b ∧ AdjNodup (b :: l) := Iff.rfl

theorem AdjNodup.tail : ∀ {a : α} {l : List α}, AdjNodup (a :: l) → AdjNodup l
  | _, [], _ => trivial
  | _, _ :: _, h => h.2

section Cat

variable [DecidableEq α]

/-- Collapse the seam: drop the head of `r` if it equals `x` (the last element
of the left operand). Both operands being canonical, one drop suffices. -/
def seam (x : α) : List α → List α
  | [] => []
  | y :: r => if x = y then r else y :: r

@[simp] theorem seam_nil (x : α) : seam x ([] : List α) = [] := rfl

theorem seam_cons (x y : α) (r : List α) :
    seam x (y :: r) = if x = y then r else y :: r := rfl

@[simp] theorem seam_cons_self (x : α) (r : List α) : seam x (x :: r) = r := by
  simp [seam]

theorem seam_cons_ne {x y : α} (h : x ≠ y) (r : List α) :
    seam x (y :: r) = y :: r := by
  simp [seam, h]

/-- Concatenation with seam collapse: the `++` of the `[T]dup` collection
(Fig 3.8). -/
def cat : List α → List α → List α
  | [], r => r
  | [a], r => a :: seam a r
  | a :: b :: l, r => a :: cat (b :: l) r

@[simp] theorem cat_nil_left (r : List α) : cat [] r = r := rfl

@[simp] theorem cat_nil_right : ∀ l : List α, cat l [] = l
  | [] => rfl
  | [_] => rfl
  | a :: b :: l => by simp [cat, cat_nil_right (b :: l)]

theorem cat_single (a : α) (r : List α) : cat [a] r = a :: seam a r := rfl

theorem cat_cons_cons (a b : α) (l r : List α) :
    cat (a :: b :: l) r = a :: cat (b :: l) r := rfl

/-- `cat` with a nonempty left operand keeps its head. -/
theorem cat_cons_exists (a : α) (l r : List α) : ∃ t, cat (a :: l) r = a :: t := by
  match l with
  | [] => exact ⟨seam a r, rfl⟩
  | b :: l' => exact ⟨cat (b :: l') r, rfl⟩

/-- `cat` is associative — on arbitrary lists, no canonicity needed. This is
the key algebraic property for at-least-once network semantics (§3.5.2). -/
theorem cat_assoc : ∀ (a b c : List α), cat (cat a b) c = cat a (cat b c)
  | [], _, _ => rfl
  | [x], b, c => by
    match b with
    | [] => simp
    | [y] =>
      by_cases hxy : x = y
      · subst hxy
        simp [cat_single, seam_cons_self]
      · simp [cat_single, seam_cons_ne hxy, cat_cons_cons]
    | y :: b' :: b'' =>
      by_cases hxy : x = y
      · subst hxy
        simp [cat_single, seam_cons_self, cat_cons_cons]
      · simp [cat_single, seam_cons_ne hxy, cat_cons_cons]
  | x :: x' :: l, b, c => by
    obtain ⟨t, ht⟩ := cat_cons_exists x' l b
    rw [cat_cons_cons x x' l b, ht, cat_cons_cons x x' t c, ← ht,
      cat_assoc (x' :: l) b c, cat_cons_cons x x' l (cat b c)]

/-- `cat` preserves canonicity. -/
theorem adjNodup_cat : ∀ {l r : List α}, AdjNodup l → AdjNodup r → AdjNodup (cat l r)
  | [], r, _, hr => hr
  | [a], r, _, hr => by
    match r with
    | [] => simp
    | y :: r' =>
      by_cases hay : a = y
      · subst hay
        simp only [cat_single, seam_cons_self]
        match r' with
        | [] => simp
        | z :: r'' => exact ⟨(hr.1 : a ≠ z), hr.2⟩
      · simp only [cat_single, seam_cons_ne hay]
        exact ⟨hay, hr⟩
  | a :: b :: l, r, hl, hr => by
    rw [cat_cons_cons]
    have ih := adjNodup_cat (l := b :: l) (r := r) hl.2 hr
    match hcat : cat (b :: l) r with
    | [] => simp
    | c :: cs =>
      refine ⟨?_, hcat ▸ ih⟩
      -- head of `cat (b :: l) r` is `b`
      have : c = b := by
        match l with
        | [] => simp [cat_single] at hcat; exact hcat.1.symm
        | _ :: _ => simp [cat_cons_cons] at hcat; exact hcat.1.symm
      exact this ▸ hl.1

end Cat

/-- A sequence with adjacent duplicates collapsed: the carrier of
`AtLeastOnce` (retry) streams. -/
structure DupList (α : Type u) [DecidableEq α] : Type u where
  toList : List α
  nodup : AdjNodup toList

namespace DupList

variable [DecidableEq α]

theorem ext : ∀ {d₁ d₂ : DupList α}, d₁.toList = d₂.toList → d₁ = d₂
  | ⟨_, _⟩, ⟨_, _⟩, rfl => rfl

/-- The empty dup-sequence. -/
def nil : DupList α := ⟨[], trivial⟩

/-- Concatenation with seam collapse. -/
def concat (d₁ d₂ : DupList α) : DupList α :=
  ⟨cat d₁.toList d₂.toList, adjNodup_cat d₁.nodup d₂.nodup⟩

@[simp] theorem concat_toList (d₁ d₂ : DupList α) :
    (d₁.concat d₂).toList = cat d₁.toList d₂.toList := rfl

@[simp] theorem concat_nil (d : DupList α) : d.concat nil = d :=
  ext (by simp [nil])

@[simp] theorem nil_concat (d : DupList α) : nil.concat d = d :=
  ext (by simp [nil])

/-- Associativity of dup-collapsing concatenation (§3.5.2). -/
theorem concat_assoc (d₁ d₂ d₃ : DupList α) :
    (d₁.concat d₂).concat d₃ = d₁.concat (d₂.concat d₃) :=
  ext (by simp [cat_assoc])

/-- Embed a single element. -/
def single (a : α) : DupList α := ⟨[a], trivial⟩

/-- The canonicalization of an arbitrary list (collapse all adjacent
duplicates): how a raw arrival sequence with retries is read into `[T]dup`. -/
def ofList : List α → DupList α
  | [] => nil
  | a :: l => (single a).concat (ofList l)

@[simp] theorem ofList_nil : (ofList [] : DupList α) = nil := rfl

theorem ofList_cons (a : α) (l : List α) :
    ofList (a :: l) = (single a).concat (ofList l) := rfl

/-- Canonicalization respects concatenation: reading chunks separately and
concatenating equals reading the joined sequence. (Eager-execution shape for
the retry channel.) -/
theorem ofList_append : ∀ (l₁ l₂ : List α),
    ofList (l₁ ++ l₂) = (ofList l₁).concat (ofList l₂)
  | [], l₂ => by simp
  | a :: l₁, l₂ => by
    simp only [List.cons_append, ofList_cons, ofList_append l₁ l₂,
      ← concat_assoc]

/-- Idempotent-fold over a dup-sequence: Hydro's `fold_idempotent`
(Fig 3.11). We fold the canonical form, so idempotence is not needed for
well-definedness — it is needed when relating a fold of raw arrivals (with
retries) to the fold of the canonical form (`foldl_canonical`). -/
def fold {β : Type v} (f : β → α → β) (init : β) (d : DupList α) : β :=
  d.toList.foldl f init

/-- Accumulator idempotence: applying the same element twice in a row is the
same as once (fold-idempotent-type, Fig 3.11). -/
def AccIdem {β : Type v} (f : β → α → β) : Prop :=
  ∀ (b : β) (a : α), f (f b a) a = f b a

/-- Seam collapse is invisible to an idempotent fold: folding `cat l r` equals
folding `l ++ r`. The collapsed seam element (if any) is exactly a consecutive
duplicate, which idempotence absorbs. -/
theorem foldl_cat {β : Type v} {f : β → α → β} (hidem : AccIdem f) :
    ∀ (l r : List α) (init : β),
      (cat l r).foldl f init = (l ++ r).foldl f init
  | [], _, _ => rfl
  | [a], r, init => by
    match r with
    | [] => rfl
    | y :: r' =>
      by_cases hay : a = y
      · subst hay
        simp only [cat_single, seam_cons_self, List.foldl, List.cons_append,
          List.nil_append]
        rw [hidem]
      · simp [cat_single, seam_cons_ne hay]
  | a :: b :: l, r, init => by
    simpa [cat_cons_cons, List.foldl] using foldl_cat hidem (b :: l) r (f init a)

/-- Folding raw arrivals (with retries) with an idempotent accumulator equals
folding the canonical dup-collapsed form: the determinism theorem for
`fold_idempotent` over retry streams (§3.5.2). -/
theorem foldl_canonical {β : Type v} {f : β → α → β} (hidem : AccIdem f) :
    ∀ (l : List α) (init : β), (ofList l).fold f init = l.foldl f init
  | [], _ => rfl
  | a :: l, init => by
    have ih := foldl_canonical hidem l (f init a)
    simp only [ofList_cons, fold, concat_toList, single, List.foldl] at *
    rw [foldl_cat hidem [a] (ofList l).toList init]
    simpa using ih

end DupList

end HydroLean
