import Hydro.Project.StepOps3

/-!
# Hydro · Project — per-operator step lemmas (`kfold_early_stop`)

The machine's early-stopping keyed fold (`keyedFoldEarlyStopAssoc`) is
an association list keyed in order of LAST arrival (`SchedKMap.ofKeyed`
dedups by last occurrence). The cell keeps one running-or-stopped
accumulator per key in that order (`kfesInsert` re-appends the key);
`kfesOf` is its closed form and `kfesSpec` the specification it meets.
-/

namespace Hydro

namespace Project

variable {L : Type} {mem : L → Nat} (pacing : Pacing L mem)

/-! ## `dedup` by last occurrence, one element at a time -/

theorem dedup_append_single {α : Type} [DecidableEq α] (a : α) :
    ∀ (l : List α), (l ++ [a]).dedup = l.dedup.erase a ++ [a]
  | [] => by simp
  | x :: l => by
    by_cases hx : x ∈ l
    · have hx' : x ∈ l ++ [a] := List.mem_append_left _ hx
      rw [List.cons_append, List.dedup_cons_of_mem hx', List.dedup_cons_of_mem hx]
      exact dedup_append_single a l
    · by_cases hxa : x = a
      · subst hxa
        have hx' : x ∈ l ++ [x] := List.mem_append_right _ (List.mem_singleton_self x)
        rw [List.cons_append, List.dedup_cons_of_mem hx', List.dedup_cons_of_notMem hx,
          List.erase_cons_head, dedup_append_single x l]
        have : l.dedup.erase x = l.dedup := List.erase_of_not_mem (by rwa [List.mem_dedup])
        rw [this]
      · have hx' : x ∉ l ++ [a] := by
          intro h
          rcases List.mem_append.mp h with h | h
          · exact hx h
          · exact hxa (List.mem_singleton.mp h)
        rw [List.cons_append, List.dedup_cons_of_notMem hx', List.dedup_cons_of_notMem hx,
          List.erase_cons_tail (by simpa using hxa), dedup_append_single a l]
        rfl

/-! ## The early-stopping run as a fold -/

/-- Stopped ⇒ the value. -/
def kfesOpt {A : Type} : A ⊕ A → Option A
  | .inr a => some a
  | .inl _ => none

theorem foldl_kfesStep_inr {V A : Type} (f : A → V → A × Bool) (a : A) :
    ∀ (vs : List V), vs.foldl (kfesStep f) (.inr a) = .inr a
  | [] => rfl
  | _ :: vs => foldl_kfesStep_inr f a vs

theorem foldEarlyStop_eq_foldl {V A : Type} (f : A → V → A × Bool) :
    ∀ (vs : List V) (acc : A), foldEarlyStop f acc vs = kfesOpt (vs.foldl (kfesStep f) (.inl acc))
  | [], _ => rfl
  | v :: vs, acc => by
    show (let r := f acc v; if r.2 then some r.1 else foldEarlyStop f r.1 vs)
      = kfesOpt (vs.foldl (kfesStep f) (let r := f acc v; if r.2 then .inr r.1 else .inl r.1))
    simp only
    split
    · rw [foldl_kfesStep_inr]; rfl
    · exact foldEarlyStop_eq_foldl f vs _

/-! ## The cell's closed form -/

/-- The cell after an arrival list: keys by last arrival, each key's
accumulator the run over its values. -/
def kfesSpec {K V A : Type} [DecidableEq K] (f : A → V → A × Bool) (init : A) (l : List (K × V)) :
    List (K × (A ⊕ A)) :=
  ((l.map Prod.fst).dedup).map (fun k => (k, (keyVals k l).foldl (kfesStep f) (.inl init)))

theorem keyVals_append_single {K V : Type} [DecidableEq K] (k : K) (l : List (K × V)) (kv : K × V) :
    keyVals k (l ++ [kv]) = keyVals k l ++ (if kv.1 = k then [kv.2] else []) := by
  unfold keyVals
  rw [List.filter_append, List.map_append]
  congr 1
  simp only [List.filter_cons, List.filter_nil]
  split <;> simp_all

theorem find?_map_key {K A : Type} [DecidableEq K] (F : K → A) (k : K) :
    ∀ (keys : List K), k ∈ keys →
      (keys.map (fun k' => (k', F k'))).find? (fun e => decide (e.1 = k)) = some (k, F k)
  | [], h => absurd h (List.not_mem_nil)
  | k' :: keys, h => by
    simp only [List.map_cons, List.find?_cons]
    by_cases hk : k' = k
    · subst hk; simp
    · simp only [hk, decide_false]
      exact find?_map_key F k keys (by
        rcases List.mem_cons.mp h with h | h
        · exact absurd h.symm hk
        · exact h)

theorem find?_map_key_none {K A : Type} [DecidableEq K] (F : K → A) (k : K) :
    ∀ (keys : List K), k ∉ keys →
      (keys.map (fun k' => (k', F k'))).find? (fun e => decide (e.1 = k)) = none
  | [], _ => rfl
  | k' :: keys, h => by
    simp only [List.map_cons, List.find?_cons]
    have hk : ¬ k' = k := fun e => h (e ▸ List.mem_cons_self ..)
    simp only [hk, decide_false]
    exact find?_map_key_none F k keys (fun hm => h (List.mem_cons_of_mem _ hm))

theorem keyVals_eq_nil_of_not_mem {K V : Type} [DecidableEq K] (k : K) (l : List (K × V))
    (h : k ∉ l.map Prod.fst) : keyVals k l = [] := by
  unfold keyVals
  rw [List.map_eq_nil_iff, List.filter_eq_nil_iff]
  intro e he hk
  exact h (List.mem_map.mpr ⟨e, he, of_decide_eq_true hk⟩)

/-- Inserting one arrival into the closed form is the closed form of the
extended list. -/
theorem kfesInsert_spec {K V A : Type} [DecidableEq K] (f : A → V → A × Bool) (init : A)
    (l : List (K × V)) (kv : K × V) :
    kfesInsertList f init (kfesSpec f init l) kv = kfesSpec f init (l ++ [kv]) := by
  unfold kfesInsertList kfesSpec
  rw [List.map_append, List.map_singleton, dedup_append_single, List.map_append, List.map_singleton]
  simp only
  -- the filtered prefix: every other key, with its (unchanged) values
  have hnd := List.nodup_dedup (l.map Prod.fst)
  congr 1
  · rw [List.filter_map, (hnd.erase_eq_filter kv.1)]
    have hfil : ((fun e : K × (A ⊕ A) => decide (e.1 ≠ kv.1)) ∘ fun k => (k, (keyVals k l).foldl (kfesStep f) (.inl init)))
        = fun x => x != kv.1 := by
      funext k
      simp only [Function.comp, ne_eq, decide_not]
      cases h : decide (k = kv.1) <;> simp_all
    rw [hfil]
    apply List.map_congr_left
    intro k hk
    have hk' : k ≠ kv.1 := by
      have := (List.mem_filter.mp hk).2
      simpa using this
    show (k, (keyVals k l).foldl (kfesStep f) (.inl init)) = (k, (keyVals k (l ++ [kv])).foldl (kfesStep f) (.inl init))
    rw [keyVals_append_single]
    have : ¬ kv.1 = k := fun e => hk' e.symm
    simp [this]
  · -- the re-appended key: its run extended by this value
    rw [keyVals_append_single, if_pos rfl, List.foldl_append]
    simp only [List.foldl_cons, List.foldl_nil]
    congr 2
    by_cases hmem : kv.1 ∈ (l.map Prod.fst).dedup
    · rw [find?_map_key _ _ _ hmem]
    · rw [find?_map_key_none _ _ _ hmem]
      rw [keyVals_eq_nil_of_not_mem _ _ (by rwa [List.mem_dedup] at hmem)]
      rfl

theorem kfesOf_spec {K V A : Type} [DecidableEq K] (f : A → V → A × Bool) (init : A) :
    ∀ (l : List (K × V)), (kfesOf f init l).1 = kfesSpec f init l := by
  intro l
  induction l using List.reverseRecOn with
  | nil => rfl
  | append_singleton l kv ih =>
    unfold kfesOf
    rw [List.foldl_append, List.foldl_cons, List.foldl_nil]
    show kfesInsertList f init (kfesOf f init l).1 kv = _
    rw [ih, kfesInsert_spec]

/-- The cell's reading is the machine's early-stopping keyed fold. -/
theorem kfesOut_spec {K V A : Type} [DecidableEq K] (f : A → V → A × Bool) (init : A) (l : List (K × V)) :
    kfesOutMap (kfesOf f init l) = keyedFoldEarlyStopAssoc f init l := by
  apply Subtype.ext
  show kfesOut (kfesOf f init l).1 = _
  rw [kfesOf_spec]
  unfold keyedFoldEarlyStopAssoc kfesSpec kfesOut SchedKMap.ofKeyed
  simp only
  rw [List.filterMap_map]
  apply List.filterMap_congr
  intro k _
  simp only [Function.comp, foldEarlyStop_eq_foldl]
  cases (keyVals k l).foldl (kfesStep f) (.inl init) <;> rfl

theorem spec_kfold_early_stop {Γ : Ctx L mem} {ℓ : L} {K V A : Type} (iK : DecidableEq K) (iV : DecidableEq V)
    (iA : DecidableEq A) (x : Var Γ (.keyed ℓ K V iK iV .totalOrder .exactlyOnce))
    (f : A → V → A × Bool) (init : A) (env : Env Γ) (t : Nat) :
    NodeSpec pacing (.kfold_early_stop iK iV iA x f init) () env t := by
  have hp : Node.prevState pacing (.kfold_early_stop iK iV iA x f init) () env t
      = fun i => kfesOf f init (prevView (env.get x i) t) := by
    cases t <;> rfl
  simp only [NodeSpec, Node.step, Node.denote, Node.stateOf, envIncAt_get, Kind.incAt]
  rw [hp]
  have hs : ∀ i, (incView (env.get x i) t).foldl (kfesInsert f init) (kfesOf f init (prevView (env.get x i) t))
      = kfesOf f init ((env.get x i).view t) := by
    intro i
    unfold kfesOf
    rw [← List.foldl_append, ← view_eq_prev_inc]
  simp only [hs]
  congr 1
  congr 1
  funext i
  exact kfesOut_spec f init _

end Project

end Hydro
