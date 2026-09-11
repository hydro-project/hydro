import HydroLean.Collections.Multiset

/-!
# Keyed views over multisets of pairs

(Metatheory substrate for Flo/Gyatso — the algebra/simp API here is kept
complete whether or not each lemma is currently consumed.)

Hydro's `KeyedStream` (`into_keyed()`, used pervasively in `collect_quorum` and
Paxos) groups a stream of `(K, V)` pairs by key and folds each group
independently. On the `NoOrder` carrier this denotes: for each key `k`, the
multiset of values paired with `k`, folded with a commutative accumulator.

Everything here is a congruent function of the underlying multiset, so keyed
aggregation of unordered network input is deterministic by construction — the
Lean counterpart of `into_keyed().fold(..., commutative = manual_proof!(...))`
in `hydro_std::quorum`.
-/

namespace HydroLean

namespace Multiset

universe u v w

variable {K : Type u} {V : Type v} {β : Type w} [DecidableEq K]

/-- The multiset of values associated with key `k`. -/
def keyVals (s : Multiset (K × V)) (k : K) : Multiset V :=
  map Prod.snd (filter (fun kv => kv.1 == k) s)

@[simp] theorem keyVals_add (s t : Multiset (K × V)) (k : K) :
    keyVals (s + t) k = keyVals s k + keyVals t k := by
  simp [keyVals]

@[simp] theorem keyVals_nil (k : K) : keyVals (nil : Multiset (K × V)) k = nil := rfl

/-- Number of pairs with key `k` (all values counted, with multiplicity). For
`collect_quorum` this is the total number of responses received for a request. -/
def countKey (s : Multiset (K × V)) (k : K) : Nat :=
  countP (fun kv => kv.1 == k) s

@[simp] theorem countKey_add (s t : Multiset (K × V)) (k : K) :
    countKey (s + t) k = countKey s k + countKey t k := countP_add _ s t

/-- Number of pairs with key `k` whose value satisfies `p`. For
`collect_quorum`: the per-key success (or error) count. -/
def countKeyP (p : V → Bool) (s : Multiset (K × V)) (k : K) : Nat :=
  countP (fun kv => kv.1 == k && p kv.2) s

@[simp] theorem countKeyP_add (p : V → Bool) (s t : Multiset (K × V)) (k : K) :
    countKeyP p (s + t) k = countKeyP p s k + countKeyP p t k := countP_add _ s t

/-- Keep only the pairs with key `k`. -/
def filterKey (s : Multiset (K × V)) (k : K) : Multiset (K × V) :=
  filter (fun kv => kv.1 == k) s

@[simp] theorem filterKey_add (s t : Multiset (K × V)) (k : K) :
    filterKey (s + t) k = filterKey s k + filterKey t k := filter_add _ s t

/-- Fold the values at key `k` with a commutative accumulator: the denotation
of Hydro's `into_keyed().fold(init, f, commutative = ...)` at a single key. -/
def foldKey (f : β → V → β) (hcomm : AccComm f) (init : β)
    (s : Multiset (K × V)) (k : K) : β :=
  foldComm f hcomm init (keyVals s k)

theorem foldKey_add (f : β → V → β) (hcomm : AccComm f) (init : β)
    (s t : Multiset (K × V)) (k : K) :
    foldKey f hcomm init (s + t) k =
      foldComm f hcomm (foldKey f hcomm init s k) (keyVals t k) := by
  simp [foldKey, foldComm_add]

/-- Remove every pair whose key appears in `ks` (Hydro's `anti_join`). -/
def antiJoin (s : Multiset (K × V)) (ks : Multiset K) : Multiset (K × V) :=
  filter (fun kv => !(ks.elem kv.1)) s

@[simp] theorem antiJoin_add (s t : Multiset (K × V)) (ks : Multiset K) :
    antiJoin (s + t) ks = antiJoin s ks + antiJoin t ks := filter_add _ s t

/-- Remove the keys of `ks` from a key multiset (Hydro's `filter_not_in`). -/
def filterNotIn (s : Multiset K) (ks : Multiset K) : Multiset K :=
  filter (fun k => !(ks.elem k)) s

@[simp] theorem filterNotIn_add (s t ks : Multiset K) :
    filterNotIn (s + t) ks = filterNotIn s ks + filterNotIn t ks := filter_add _ s t

/-! ### Relating the keyed observables -/

theorem countKey_eq_card_keyVals (s : Multiset (K × V)) (k : K) :
    countKey s k = card (keyVals s k) := by
  induction s using Quotient.ind with | _ l =>
  simp only [countKey, quot_mk_eq_ofList, countP_ofList, keyVals, filter_ofList,
    map_ofList, card_ofList, List.length_map, ← List.countP_eq_length_filter]

theorem countKeyP_eq_countP_keyVals (p : V → Bool) (s : Multiset (K × V)) (k : K) :
    countKeyP p s k = countP p (keyVals s k) := by
  induction s using Quotient.ind with | _ l =>
  simp only [countKeyP, quot_mk_eq_ofList, countP_ofList, keyVals, filter_ofList,
    map_ofList]
  induction l with
  | nil => rfl
  | cons kv l ih =>
    by_cases hk : kv.1 == k <;> by_cases hp : p kv.2 <;>
      simp [hk, hp, ih]

end Multiset

end HydroLean
