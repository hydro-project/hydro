import Hydro.HydroTick

/-!
# Quickstart: the quorum's vocabulary, records and contract

Everything `vote_quorum` (`Examples/Tutorials/Quorum.lean`) and its tutorial stages
(`Examples/TutorialSteps/`) share: the counts the contract speaks of, the coordinator's per-key counter
`yesStep` and its commutativity proof, the lemmas that read the keyed fold back into those
counts, the decision and schedule records, and the contract `QuorumEnsures`.
-/

namespace Hydro

namespace Tutorials

variable {L : Type} {mem : L → Nat}

/-! ## Vocabulary -/

--[vocab]--
/-- The yes votes for request `p` in a pool of `(request, vote)` pairs. -/
def yesVotes (pool : Multiset (Nat × Bool)) (p : Nat) : Nat :=
  (pool.filter (fun r => r.1 = p ∧ r.2 = true)).card

/-- Everything the coordinator members were asked to decide, pooled. -/
def proposed {n : Nat} (requests : Fin n → Multiset Nat) : Multiset Nat :=
  ((List.finRange n).map requests).sum

/-- The participants that vote yes on `p`. -/
def yesVoters {n : Nat} (vote : Fin n → Nat → Bool) (p : Nat) : Nat :=
  ((List.finRange n).filter (fun j => vote j p)).length
--[/vocab]--

--[yesStep]--
/-- The coordinator's per-key counter: one more for each yes vote. -/
def yesStep (n : Nat) (v : Bool) : Nat := if v then n + 1 else n

theorem yesStep_comm (s : Nat) (x y : Bool) :
    yesStep (yesStep s x) y = yesStep (yesStep s y) x := by
  cases x <;> cases y <;> rfl
--[/yesStep]--

theorem yesVotes_add (a b : Multiset (Nat × Bool)) (p : Nat) :
    yesVotes (a + b) p = yesVotes a p + yesVotes b p := by
  unfold yesVotes
  rw [Multiset.filter_add, Multiset.card_add]

theorem yesVotes_mono {a b : Multiset (Nat × Bool)} (h : a ≤ b) (p : Nat) :
    yesVotes a p ≤ yesVotes b p :=
  Multiset.card_le_card (Multiset.filter_le_filter _ h)

theorem yesVotes_listSum (l : List (Multiset (Nat × Bool))) (p : Nat) :
    yesVotes l.sum p = (l.map (fun m => yesVotes m p)).sum := by
  induction l with
  | nil => rfl
  | cons m t ih => rw [List.sum_cons, yesVotes_add, ih, List.map_cons, List.sum_cons]

/-- One participant's votes on a request pool: its yes votes for `p` are the copies of `p` in
the request pool, if it votes yes on `p`. -/
theorem yesVotes_map_tag (m : Multiset Nat) (b : Nat → Bool) (p : Nat) :
    yesVotes (m.map (fun q => (q, b q))) p = if b p then m.count p else 0 := by
  induction m using Multiset.induction_on with
  | empty => split <;> rfl
  | cons a m ih =>
    unfold yesVotes at ih ⊢
    rw [Multiset.map_cons, Multiset.filter_cons, Multiset.card_add, ih, Multiset.count_cons]
    by_cases ha : a = p
    · subst ha
      by_cases hb : b a = true <;> simp [hb]
      omega
    · simp [ha, Ne.symm ha]

theorem sum_map_ite_const {ι : Type _} (l : List ι) (q : ι → Bool) (c : Nat) :
    (l.map (fun j => if q j then c else 0)).sum = (l.filter q).length * c := by
  induction l with
  | nil => simp
  | cons a t ih =>
    rw [List.map_cons, List.sum_cons, ih, List.filter_cons]
    by_cases h : q a
    · rw [if_pos h, if_pos h, List.length_cons, Nat.succ_mul, Nat.add_comm]
    · rw [if_neg h, if_neg h, Nat.zero_add]

/-- The yes counter folds to the number of yes votes. -/
theorem foldl_yesStep (vs : Multiset Bool) (a : Nat) :
    @Multiset.foldl Bool Nat yesStep ⟨fun s x y => yesStep_comm s x y⟩ a vs
      = a + (vs.filter (fun v => v = true)).card := by
  induction vs using Multiset.induction_on generalizing a with
  | empty => rfl
  | cons v vs ih =>
    rw [Multiset.foldl_cons, ih, Multiset.filter_cons]
    cases v
    · simp [yesStep]
    · simp [yesStep]; omega

/-- The coordinator's keyed counter (`into_keyed().fold(0, …)` read under `den`). -/
abbrev yesCounter (pool : Multiset (Nat × Bool)) : KeyedMap Nat Nat :=
  KeyedMap.ofFold yesStep 0 (fun _ _ => True) (fun s x y _ => yesStep_comm s x y)
    (KeyedPool.ofMultiset pool)

theorem yesCounter_get (pool : Multiset (Nat × Bool)) (p : Nat) :
    (yesCounter pool).get p
      = if p ∈ (pool.map Prod.fst).dedup then some (yesVotes pool p) else none := by
  show (if p ∈ (pool.map Prod.fst).toFinset then some _ else none) = _
  by_cases hp : p ∈ (pool.map Prod.fst).dedup
  · rw [if_pos (Multiset.mem_toFinset.mpr (Multiset.mem_dedup.mp hp)), if_pos hp]
    congr 1
    show Multiset.foldlOn (fun _ _ => True) yesStep 0 (fun s x y _ => yesStep_comm s x y)
      ((pool.filter (fun e => e.1 = p)).map Prod.snd) = _
    rw [Multiset.foldlOn_true, foldl_yesStep, Nat.zero_add]
    unfold yesVotes
    rw [Multiset.filter_map, Multiset.card_map, Multiset.filter_filter]
    congr 2
    funext r
    exact propext and_comm
  · rw [if_neg (fun h => hp (Multiset.mem_dedup.mpr (Multiset.mem_toFinset.mp h))), if_neg hp]

/-- A request the counter reports at `minYes` or more has `minYes` yes votes in the pool. -/
theorem le_yesVotes_of_mem_reached {minYes : Nat} {pool : Multiset (Nat × Bool)} {p : Nat}
    (h : p ∈ ((yesCounter pool).filter fun n => decide (minYes ≤ n)).keysMS) :
    minYes ≤ yesVotes pool p := by
  obtain ⟨-, hany⟩ := KeyedMap.mem_keysMS_filter.mp h
  rw [yesCounter_get] at hany
  split at hany
  · simpa using hany
  · exact absurd hany (by simp)

/-- The reached requests are each listed once. -/
theorem reached_nodup (minYes : Nat) (pool : Multiset (Nat × Bool)) :
    ((yesCounter pool).filter fun n => decide (minYes ≤ n)).keysMS.Nodup :=
  Finset.nodup _

/-! ## The records and the contract -/

--[records]--
/-- The program's one `nondet!`: where the coordinator's batches of votes end. -/
structure QuorumDec (H : HydroSem L mem) (nCoord : Nat) where
  votes : H.BatchDec nCoord (Nat × Bool) .noOrder .exactlyOnce

/-- The program's two network edges: `Unit` at `Values`, delivery cursors on the machine. -/
structure QuorumSched (H : HydroSem L mem) (nCoord nPart : Nat) where
  toParticipants : H.TransportDec nPart nCoord
  toCoordinator : H.TransportDec nCoord nPart
--[/records]--

--[ensures]--
/-- What `vote_quorum` ensures, over the `Values` denotation: per coordinator member `c`, its
decisions `out c`, for any batching of the votes. -/
structure QuorumEnsures {nCoord nPart : Nat} (minYes : Nat) (vote : Fin nPart → Nat → Bool)
    (requests : Fin nCoord → Multiset Nat) (out : Fin nCoord → Multiset Nat) : Prop where
  /-- A decided request was proposed. -/
  decided_proposed : ∀ c, ∀ p ∈ out c, 1 ≤ minYes → p ∈ proposed requests
  /-- A request proposed at most once is decided only if `minYes` participants voted yes. -/
  quorum : ∀ c, ∀ p ∈ out c, (proposed requests).count p ≤ 1 → minYes ≤ yesVoters vote p
  /-- A request is decided at most once. -/
  once : ∀ c p, (out c).count p ≤ 1
--[/ensures]--

end Tutorials

end Hydro
