import HydroLean.Prelude

/-!
# Multisets as quotients of lists by permutation

(Metatheory substrate for Flo/Gyatso — the algebra/simp API here is kept
complete whether or not each lemma is currently consumed.)

The dissertation's `[T]unord` collection (§3.5.2, Fig 3.10) captures streams
whose element *order* has been erased: reliable-but-unordered network delivery
(e.g. QUIC) is a **deterministic** function into this quotient, so downstream
logic that only uses congruent operations never observes the nondeterministic
arrival order. This is the central mechanism by which Hydro *defers
materializing* network nondeterminism.

We define `HydroLean.Multiset α := Quotient (List.isSetoid α)` (core Lean's
permutation setoid) and develop the API needed by the Hydro surface layer:
`add` (concatenation of received chunks), `count`/`countP` (deterministic
cardinalities, §3.5.3: "the total cardinality of each element is
deterministic"), `map`/`filter`/`filterMap` (congruent element-wise operators),
and `foldComm` — the formal counterpart of Hydro's `fold_commutative`
(Fig 3.11): a fold over a multiset is well-defined *exactly when* the
accumulator function is commutative, which here is enforced by `Quotient.lift`
rather than by code review of a `manual_proof!` annotation.
-/

namespace HydroLean

universe u v

/-- A multiset: a list up to permutation. The carrier of unordered streams
(`NoOrder` marker in Hydro, `[T]unord` in §3.5.2). -/
def Multiset (α : Type u) : Type u := Quotient (List.isSetoid α)

namespace Multiset

variable {α : Type u} {β : Type v}

/-- Erase the order of a list. On streams: the coercion from a `TotalOrder`
stream to a `NoOrder` stream (Hydro's safe marker weakening). -/
def ofList (l : List α) : Multiset α := Quotient.mk _ l

@[simp] theorem quot_mk_eq_ofList (l : List α) :
    (Quotient.mk (List.isSetoid α) l) = ofList l := rfl

/-- The empty multiset. -/
def nil : Multiset α := ofList []

instance : EmptyCollection (Multiset α) := ⟨nil⟩
instance : Inhabited (Multiset α) := ⟨nil⟩

theorem sound {l₁ l₂ : List α} (h : l₁.Perm l₂) : ofList l₁ = ofList l₂ :=
  Quotient.sound h

theorem exact {l₁ l₂ : List α} (h : ofList l₁ = ofList l₂) : l₁.Perm l₂ :=
  Quotient.exact h

/-- Every multiset is `ofList` of some list. -/
theorem exists_rep (s : Multiset α) : ∃ l : List α, ofList l = s :=
  Quotient.exists_rep s

/-- Prepend an element. -/
def cons (a : α) (s : Multiset α) : Multiset α :=
  Quotient.lift (fun l => ofList (a :: l))
    (fun _ _ h => sound (List.Perm.cons a h)) s

@[simp] theorem cons_ofList (a : α) (l : List α) :
    cons a (ofList l) = ofList (a :: l) := rfl

/-- Union / concatenation of multisets. This is the `++` of the `[T]unord`
collection: arrival of a new chunk of unordered data (§3.5.2, Fig 3.10). -/
def add (s t : Multiset α) : Multiset α :=
  Quotient.lift₂ (fun l₁ l₂ => ofList (l₁ ++ l₂))
    (fun _ _ _ _ h₁ h₂ => sound (List.Perm.append h₁ h₂)) s t

instance : Add (Multiset α) := ⟨add⟩

@[simp] theorem add_ofList (l₁ l₂ : List α) :
    ofList l₁ + ofList l₂ = ofList (l₁ ++ l₂) := rfl

/-- Multiset union is commutative: the order in which unordered chunks arrive
is unobservable. -/
theorem add_comm (s t : Multiset α) : s + t = t + s := by
  induction s using Quotient.ind
  induction t using Quotient.ind
  exact sound List.perm_append_comm

theorem add_assoc (s t u : Multiset α) : s + t + u = s + (t + u) := by
  induction s using Quotient.ind
  induction t using Quotient.ind
  induction u using Quotient.ind
  simp only [quot_mk_eq_ofList, add_ofList, List.append_assoc]

@[simp] theorem add_nil (s : Multiset α) : s + nil = s := by
  induction s using Quotient.ind
  simp only [quot_mk_eq_ofList, nil, add_ofList, List.append_nil]

@[simp] theorem nil_add (s : Multiset α) : nil + s = s := by
  induction s using Quotient.ind
  rfl

theorem cons_eq_add_single (a : α) (s : Multiset α) :
    cons a s = ofList [a] + s := by
  induction s using Quotient.ind
  rfl

/-- Number of elements. Deterministic regardless of arrival order. -/
def card (s : Multiset α) : Nat :=
  Quotient.lift List.length (fun _ _ h => List.Perm.length_eq h) s

@[simp] theorem card_ofList (l : List α) : card (ofList l) = l.length := rfl

@[simp] theorem card_add (s t : Multiset α) : card (s + t) = card s + card t := by
  induction s using Quotient.ind
  induction t using Quotient.ind
  simp only [quot_mk_eq_ofList, add_ofList, card_ofList, List.length_append]

/-- Membership. -/
def Mem (s : Multiset α) (a : α) : Prop :=
  Quotient.lift (fun l => a ∈ l) (fun _ _ h => propext (List.Perm.mem_iff h)) s

instance : Membership α (Multiset α) := ⟨Mem⟩

@[simp] theorem mem_ofList {a : α} {l : List α} : a ∈ ofList l ↔ a ∈ l :=
  Iff.rfl

/-- Count elements satisfying a Boolean predicate. This is the deterministic
observable of an unordered stream (§3.5.3: cardinalities are deterministic even
though order is not). -/
def countP (p : α → Bool) (s : Multiset α) : Nat :=
  Quotient.lift (List.countP p) (fun _ _ h => List.Perm.countP_eq p h) s

@[simp] theorem countP_ofList (p : α → Bool) (l : List α) :
    countP p (ofList l) = l.countP p := rfl

@[simp] theorem countP_add (p : α → Bool) (s t : Multiset α) :
    countP p (s + t) = countP p s + countP p t := by
  induction s using Quotient.ind
  induction t using Quotient.ind
  simp only [quot_mk_eq_ofList, add_ofList, countP_ofList, List.countP_append]

section Count

variable [DecidableEq α]

/-- Multiplicity of an element. -/
def count (a : α) (s : Multiset α) : Nat := countP (fun x => x == a) s

theorem count_eq_list_count (a : α) (l : List α) :
    count a (ofList l) = l.count a := by
  simp only [count, countP_ofList, List.count_eq_countP]

/-- `count` on `ofList` is `List.count`. -/
theorem count_ofList (a : α) (l : List α) : count a (ofList l) = l.count a :=
  count_eq_list_count a l

@[simp] theorem count_nil (a : α) : count a (nil : Multiset α) = 0 := rfl

@[simp] theorem count_add (a : α) (s t : Multiset α) :
    count a (s + t) = count a s + count a t := countP_add _ s t

/-- Boolean membership test (decidable `∈`), needed for anti-joins. -/
def elem (s : Multiset α) (a : α) : Bool := count a s != 0

theorem elem_iff_mem {s : Multiset α} {a : α} : s.elem a = true ↔ a ∈ s := by
  induction s using Quotient.ind with | _ l =>
  simp only [elem, count_eq_list_count, quot_mk_eq_ofList, mem_ofList,
    bne_iff_ne, ne_eq, ← List.count_pos_iff (l := l) (a := a)]
  omega

/-- Extensionality: multisets are determined by their multiplicities. -/
theorem ext {s t : Multiset α} (h : ∀ a, count a s = count a t) : s = t := by
  induction s using Quotient.ind with | _ ls =>
  induction t using Quotient.ind with | _ lt =>
  refine Quotient.sound (List.perm_iff_count.mpr fun a => ?_)
  have := h a
  simpa [count_eq_list_count] using this

end Count

/-- Membership distributes over union. -/
theorem mem_add {a : α} {s t : Multiset α} : a ∈ s + t ↔ a ∈ s ∨ a ∈ t := by
  induction s using Quotient.ind
  induction t using Quotient.ind
  simp only [quot_mk_eq_ofList, add_ofList, mem_ofList, List.mem_append]

/-- Map a function over a multiset (Hydro's `map` on `NoOrder` streams:
element-wise operators are congruent, so they preserve determinism, §4.3.2). -/
def map (f : α → β) (s : Multiset α) : Multiset β :=
  Quotient.lift (fun l => ofList (l.map f)) (fun _ _ h => sound (List.Perm.map f h)) s

@[simp] theorem map_ofList (f : α → β) (l : List α) :
    map f (ofList l) = ofList (l.map f) := rfl

@[simp] theorem map_add (f : α → β) (s t : Multiset α) :
    map f (s + t) = map f s + map f t := by
  induction s using Quotient.ind
  induction t using Quotient.ind
  simp only [quot_mk_eq_ofList, add_ofList, map_ofList, List.map_append]

/-- Filter by a Boolean predicate. -/
def filter (p : α → Bool) (s : Multiset α) : Multiset α :=
  Quotient.lift (fun l => ofList (l.filter p)) (fun _ _ h => sound (List.Perm.filter p h)) s

@[simp] theorem filter_ofList (p : α → Bool) (l : List α) :
    filter p (ofList l) = ofList (l.filter p) := rfl

@[simp] theorem filter_add (p : α → Bool) (s t : Multiset α) :
    filter p (s + t) = filter p s + filter p t := by
  induction s using Quotient.ind
  induction t using Quotient.ind
  simp only [quot_mk_eq_ofList, add_ofList, filter_ofList, List.filter_append]

/-- Simultaneous filter and map (Hydro's `filter_map`). -/
def filterMap (f : α → Option β) (s : Multiset α) : Multiset β :=
  Quotient.lift (fun l => ofList (l.filterMap f))
    (fun _ _ h => sound (List.Perm.filterMap f h)) s

@[simp] theorem filterMap_ofList (f : α → Option β) (l : List α) :
    filterMap f (ofList l) = ofList (l.filterMap f) := rfl

@[simp] theorem filterMap_add (f : α → Option β) (s t : Multiset α) :
    filterMap f (s + t) = filterMap f s + filterMap f t := by
  induction s using Quotient.ind
  induction t using Quotient.ind
  simp only [quot_mk_eq_ofList, add_ofList, filterMap_ofList, List.filterMap_append]

/-! ## Commutative folds

`List.foldl` is invariant under permutation of the input *iff* the accumulator
function is commutative in its second argument. Hydro exposes this as
`fold_commutative` (§4.3.4, Fig 3.11), asking the developer to promise
commutativity via `manual_proof!`; here the promise is a real hypothesis and
the fold is defined by `Quotient.lift`, so using a non-commutative function on
an unordered stream is not merely unsafe — it is *untypeable*. -/

/-- `f` is commutative as an accumulator: folding two elements in either order
yields the same state. This is the algebraic side condition of
`fold_commutative` (Fig 3.11, fold-commutative-type). -/
def AccComm (f : β → α → β) : Prop :=
  ∀ (b : β) (a₁ a₂ : α), f (f b a₁) a₂ = f (f b a₂) a₁

/-- `List.foldl` is permutation-invariant for commutative accumulators. -/
theorem foldl_perm {f : β → α → β} (hcomm : AccComm f) {l₁ l₂ : List α}
    (h : l₁.Perm l₂) : ∀ b : β, l₁.foldl f b = l₂.foldl f b := by
  induction h with
  | nil => intro b; rfl
  | cons x _ ih => intro b; simpa using ih (f b x)
  | swap x y l => intro b; simp [List.foldl, hcomm b]
  | trans _ _ ih₁ ih₂ => intro b; exact (ih₁ b).trans (ih₂ b)

/-- Fold a multiset with a commutative accumulator (Hydro's
`fold_commutative`). Well-definedness on the quotient *is* the determinism
argument of §3.5.2. -/
def foldComm (f : β → α → β) (hcomm : AccComm f) (init : β) (s : Multiset α) : β :=
  Quotient.lift (fun l => l.foldl f init)
    (fun _ _ h => foldl_perm hcomm h init) s

@[simp] theorem foldComm_ofList (f : β → α → β) (hcomm : AccComm f) (init : β)
    (l : List α) : foldComm f hcomm init (ofList l) = l.foldl f init := rfl

@[simp] theorem foldComm_nil (f : β → α → β) (hcomm : AccComm f) (init : β) :
    foldComm f hcomm init nil = init := rfl

theorem foldComm_cons (f : β → α → β) (hcomm : AccComm f) (init : β) (a : α)
    (s : Multiset α) :
    foldComm f hcomm init (cons a s) = foldComm f hcomm (f init a) s := by
  induction s using Quotient.ind
  rfl

/-- Folding a union folds the pieces in sequence — the incremental-execution
property: chunks may arrive and be folded eagerly, and the settled result is
the fold of the whole input (eager execution, Def 2.3.1, for this operator). -/
theorem foldComm_add (f : β → α → β) (hcomm : AccComm f) (init : β)
    (s t : Multiset α) :
    foldComm f hcomm init (s + t) = foldComm f hcomm (foldComm f hcomm init s) t := by
  induction s using Quotient.ind
  induction t using Quotient.ind
  simp only [quot_mk_eq_ofList, add_ofList, foldComm_ofList, List.foldl_append]

/-- Induction: every multiset is built from `nil` by `cons`. -/
theorem inductionOn {P : Multiset α → Prop} (s : Multiset α)
    (hnil : P nil) (hcons : ∀ (a : α) (t : Multiset α), P t → P (cons a t)) :
    P s := by
  induction s using Quotient.ind with | _ l =>
  induction l with
  | nil => exact hnil
  | cons a l ih => exact hcons a (ofList l) ih

/-- A multiset has no duplicates (well-defined because permutation preserves
`List.Nodup`). Promoted from the JoinResponses model (dedup pass); the
`HydroLean.Programs.Multiset.MNodup` name remains as an alias. -/
def MNodup (s : Multiset α) : Prop :=
  Quotient.lift List.Nodup (fun _ _ h => propext (List.Perm.nodup_iff h)) s

@[simp] theorem mnodup_ofList (l : List α) :
    MNodup (ofList l) ↔ l.Nodup := Iff.rfl

end Multiset

end HydroLean
