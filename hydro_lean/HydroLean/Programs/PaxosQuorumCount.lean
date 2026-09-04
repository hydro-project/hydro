import HydroLean.Programs.CollectQuorumWithResponse
import HydroLean.Programs.PaxosQuorumModel

/-!
# Per-(key, value) count soundness for `collect_quorum_with_response`

`PaxosQuorumModel.lean` provides *key-level* emission soundness for the quorum
loops (an emitted key had ≥ `min` successes). The Paxos agreement proof needs
one more unconditional fact, at the **(key, value) multiplicity** level: the
number of `(k, v)` pairs ever emitted never exceeds the number of `(k, .ok v)`
inputs ever consumed (`wrRun_outCnt_le`). This is what turns "the leader
collected `f+1` payloads for ballot `b`" into "`f+1` **distinct acceptors**
promised `b`": each acceptor contributes at most one `Ok` response per ballot
(guarded variant), so `f+1` output positions inject into `f+1` distinct
acceptors by counting within each `(k, v)` class — no trajectory enumeration
and no positional bookkeeping needed.

The invariant `WRCS` is the emit-once accounting of the loop
(quorum.rs:22–87): emissions so far, plus the still-unemitted retained
responses — discounted for keys parked in `min_but_not_max`, whose retained
responses were already emitted — never exceed consumption; and parked keys
retain a full quorum of successes (which is why they stay parked until
dropped, never re-emitting).
-/

namespace HydroLean.Programs

open HydroLean.Hydro

universe u v w

variable {κ : Type u} {V : Type v} {E : Type w} [DecidableEq κ] [DecidableEq V]

/-- The `(k, v)`-class indicator on outputs. -/
def outIs (k : κ) (v : V) : κ × V → Bool :=
  fun e => decide (e.1 = k) && decide (e.2 = v)

/-- The `(k, .ok v)`-class indicator on inputs (no `DecidableEq E` needed). -/
def okIs (k : κ) (v : V) : κ × Except E V → Bool :=
  fun e => decide (e.1 = k) &&
    (match e.2 with | .ok w => decide (w = v) | .error _ => false)

/-- Count of `(k, v)` pairs in an output stream. -/
def outCnt (k : κ) (v : V) (l : Stream (κ × V)) : Nat :=
  l.countP (outIs k v)

/-- Count of `(k, .ok v)` responses in an input stream. -/
def okCnt (k : κ) (v : V) (l : Stream (κ × Except E V)) : Nat :=
  l.countP (okIs (E := E) k v)

theorem outCnt_append (k : κ) (v : V) (l₁ l₂ : Stream (κ × V)) :
    outCnt k v (l₁ ++ l₂) = outCnt k v l₁ + outCnt k v l₂ :=
  List.countP_append ..

theorem okCnt_append (k : κ) (v : V) (l₁ l₂ : Stream (κ × Except E V)) :
    okCnt k v (l₁ ++ l₂) = okCnt k v l₁ + okCnt k v l₂ :=
  List.countP_append ..

/-- The `emit` filterMap preserves class counts: emitted `(k, v)` pairs of a
source are exactly its `(k, .ok v)` responses. Stated for an *abstract*
emission function with a pointwise hypothesis, so that use sites unify `f`
with whatever match-compiler auxiliary the tick's `emit` elaborated to (the
hypothesis is then discharged by `cases`, which iota-reduces any matcher). -/
theorem outCnt_emit (k : κ) (v : V) (l : Stream (κ × Except E V))
    (f : κ × Except E V → Option (κ × V))
    (hf : ∀ e, Option.elim (f e) false (outIs k v) = okIs (E := E) k v e) :
    outCnt k v (l.filterMap f) = okCnt k v l := by
  induction l with
  | nil => rfl
  | cons e rest ih =>
    unfold outCnt okCnt at ih ⊢
    simp only [Stream.filterMap] at ih ⊢
    rw [List.filterMap_cons]
    cases hfe : f e with
    | none =>
      rw [List.countP_cons]
      have hz : okIs (E := E) k v e = false := by
        rw [← hf e, hfe]
        rfl
      rw [hz]
      simpa using ih
    | some p =>
      rw [List.countP_cons, List.countP_cons, ih]
      congr 1
      rw [← hf e, hfe]
      rfl

/-- Discharges `outCnt_emit`'s pointwise hypothesis for any elaboration of
the quorum loops' `emit` function (quorum.rs:74–77). -/
theorem emit_pointwise (k : κ) (v : V)
    (f : κ × Except E V → Option (κ × V))
    (hok : ∀ (c : κ) (w : V), f (c, .ok w) = some (c, w))
    (herr : ∀ (c : κ) (err : E), f (c, .error err) = none) :
    ∀ e, Option.elim (f e) false (outIs k v) = okIs (E := E) k v e := by
  rintro ⟨c, r⟩
  cases r with
  | ok w =>
    rw [hok]
    rfl
  | error err =>
    rw [herr]
    show false = okIs (E := E) k v (c, .error err)
    rw [okIs]
    show false = (decide (c = k) && false)
    rw [Bool.and_false]

/-- `okCnt` through `antiJoin`: zero when the key is excluded, unchanged
otherwise. -/
theorem okCnt_antiJoin (k : κ) (v : V) (l : Stream (κ × Except E V))
    (ks : Stream κ) :
    okCnt k v (l.antiJoin ks) = if k ∈ ks then 0 else okCnt k v l := by
  unfold okCnt Stream.antiJoin Stream.filter
  rw [List.countP_filter]
  by_cases hk : k ∈ ks
  · rw [if_pos hk]
    refine List.countP_eq_zero.mpr fun e he hcon => ?_
    rw [Bool.and_eq_true] at hcon
    obtain ⟨hokis, hnc⟩ := hcon
    rw [okIs, Bool.and_eq_true, decide_eq_true_iff] at hokis
    have hc : ks.contains e.1 = true := by
      rw [hokis.1]
      exact List.elem_eq_true_of_mem hk
    rw [hc] at hnc
    cases hnc
  · rw [if_neg hk]
    refine List.countP_congr fun e _ => ?_
    have heq : (okIs k v e && !ks.contains e.1) = okIs (E := E) k v e := by
      by_cases h1 : e.1 = k
      · have hc : ks.contains e.1 = false := by
          rw [h1]
          exact Bool.eq_false_iff.mpr fun h => hk (List.contains_iff_mem.mp h)
        rw [hc]
        simp
      · have hz : okIs (E := E) k v e = false := by
          rw [okIs, Bool.and_eq_false_iff]
          exact Or.inl (by simpa using h1)
        rw [hz]
        simp
    rw [heq]

/-- `okCnt` through a key-predicate filter. -/
theorem okCnt_filter_key (k : κ) (v : V) (l : Stream (κ × Except E V))
    (p : κ → Bool) :
    okCnt k v (l.filter fun e => p e.1) = if p k then okCnt k v l else 0 := by
  unfold okCnt Stream.filter
  rw [List.countP_filter]
  by_cases hp : p k
  · rw [if_pos hp]
    refine List.countP_congr fun e _ => ?_
    have heq : (okIs k v e && p e.1) = okIs (E := E) k v e := by
      by_cases h1 : e.1 = k
      · rw [h1, hp]
        simp
      · have hz : okIs (E := E) k v e = false := by
          rw [okIs, Bool.and_eq_false_iff]
          exact Or.inl (by simpa using h1)
        rw [hz]
        simp
    rw [heq]
  · rw [if_neg hp]
    refine List.countP_eq_zero.mpr fun e he hcon => ?_
    rw [Bool.and_eq_true] at hcon
    obtain ⟨hokis, hpe⟩ := hcon
    rw [okIs, Bool.and_eq_true, decide_eq_true_iff] at hokis
    rw [hokis.1] at hpe
    exact hp hpe

/-- `countKeyP` through `antiJoin` (key-class version of `okCnt_antiJoin`). -/
theorem countKeyP_antiJoin' (k : κ) (pr : Except E V → Bool)
    (l : Stream (κ × Except E V)) (ks : Stream κ) :
    (l.antiJoin ks).countKeyP k pr
      = if k ∈ ks then 0 else l.countKeyP k pr := by
  unfold Stream.countKeyP Stream.antiJoin Stream.filter
  rw [List.countP_filter]
  by_cases hk : k ∈ ks
  · rw [if_pos hk]
    refine List.countP_eq_zero.mpr fun e he hcon => ?_
    rw [Bool.and_eq_true, Bool.and_eq_true] at hcon
    obtain ⟨⟨h1, -⟩, hnc⟩ := hcon
    rw [decide_eq_true_iff] at h1
    have hc : ks.contains k = true := List.elem_eq_true_of_mem hk
    rw [h1, hc] at hnc
    cases hnc
  · rw [if_neg hk]
    refine List.countP_congr fun e _ => ?_
    have heq : ((decide (e.1 = k) && pr e.2) && !ks.contains e.1)
        = (decide (e.1 = k) && pr e.2) := by
      by_cases h1 : e.1 = k
      · have hc : ks.contains e.1 = false := by
          rw [h1]
          exact Bool.eq_false_iff.mpr fun h => hk (List.contains_iff_mem.mp h)
        rw [hc]
        simp
      · have : (decide (e.1 = k)) = false := by simpa using h1
        rw [this]
        simp
    rw [heq]

/-! ## The emit-once accounting invariant -/

/-- The WR loop's accounting invariant, over the consumed input so far:
1. emissions + still-unemitted retained responses (discounted for parked
   keys) never exceed consumption, per `(k, v)` class;
2. parked keys (`min_but_not_max`) retain a full quorum of successes — so
   they stay parked (never re-emitting) until dropped;
3. the `min = max` branch never parks. -/
structure WRCS (min max : Nat) (consumed : Stream (κ × Except E V))
    (st : QuorumWRState κ V E) (out : Stream (κ × V)) : Prop where
  count : ∀ (k : κ) (v : V),
    outCnt k v out +
        (if k ∈ st.minButNotMax then 0 else okCnt k v st.notAll)
      ≤ okCnt k v consumed
  parked : ∀ k ∈ st.minButNotMax, min ≤ st.notAll.countKeyP k Except.isOk
  mbnmEmpty : min = max → st.minButNotMax = []

theorem wrcs_init (min max : Nat) :
    WRCS min max ([] : Stream (κ × Except E V)) ⟨[], []⟩ [] := by
  refine { count := fun k v => ?_, parked := fun k hk => (nomatch hk),
           mbnmEmpty := fun _ => rfl }
  show outCnt k v [] + (if k ∈ ([] : List κ) then 0 else okCnt k v []) ≤
    okCnt k v []
  rw [if_neg (List.not_mem_nil)]
  show (0 : Nat) + 0 ≤ 0
  simp

/-- Membership in `reached_min_count` characterized by the success count
(for `1 ≤ min`). -/
theorem mem_reachedMin_iff {min : Nat} (hmin : 1 ≤ min)
    (cur : Stream (κ × Except E V)) (k : κ) :
    k ∈ cur.keys.filter
        (fun k' => decide (min ≤ cur.countKeyP k' Except.isOk))
      ↔ min ≤ cur.countKeyP k Except.isOk := by
  constructor
  · intro h
    have := (List.mem_filter.mp h).2
    simpa using this
  · intro h
    refine List.mem_filter.mpr ⟨?_, by simpa using h⟩
    have hpos : 0 < cur.countKeyP k Except.isOk := Nat.lt_of_lt_of_le hmin h
    exact Stream.mem_keys.mpr (Stream.mem_map_fst_of_countKeyP_pos hpos)

/-- One WR tick preserves the accounting invariant. -/
theorem wrcs_step {min max : Nat} (hmin : 1 ≤ min)
    {consumed b : Stream (κ × Except E V)} {st : QuorumWRState κ V E}
    {out : Stream (κ × V)} (h : WRCS min max consumed st out) :
    WRCS min max (consumed ++ b)
      ((collectQuorumWRTick min max).step st b).1
      (out ++ ((collectQuorumWRTick min max).step st b).2) := by
  classical
  simp only [collectQuorumWRTick]
  -- abstract the tick's `current_responses` (also under the binders)
  generalize hcur : st.notAll.chain b = cur
  have hcurCnt : ∀ (k : κ) (v : V),
      okCnt k v cur = okCnt k v st.notAll + okCnt k v b := by
    intro k v
    rw [← hcur]
    exact okCnt_append k v st.notAll b
  have hcurKP : ∀ (k : κ),
      cur.countKeyP k Except.isOk
        = st.notAll.countKeyP k Except.isOk + b.countKeyP k Except.isOk := by
    intro k
    rw [← hcur]
    exact Stream.countKeyP_append st.notAll b k Except.isOk
  by_cases hmm : min = max
  · -- `min = max` branch: no parking, emit-or-retain per key
    rw [if_pos hmm]
    dsimp only
    have hempty := h.mbnmEmpty hmm
    refine { count := fun k v => ?_,
             parked := fun k hk => (nomatch hk),
             mbnmEmpty := fun _ => rfl }
    rw [outCnt_append, okCnt_append,
      outCnt_emit k v _ _ (emit_pointwise k v _ (fun c w => rfl)
        (fun c err => rfl)),
      okCnt_filter_key k v cur
        (fun k' => !decide (cur.countKeyP k' Except.isOk < min)),
      if_neg List.not_mem_nil, okCnt_antiJoin]
    have hold := h.count k v
    rw [if_neg (by rw [hempty]; exact List.not_mem_nil)] at hold
    by_cases hs : min ≤ cur.countKeyP k Except.isOk
    · rw [if_pos (by simpa using Nat.not_lt.mpr hs),
        if_pos ((mem_reachedMin_iff hmin cur k).mpr hs), hcurCnt]
      omega
    · rw [if_neg (by simpa using Nat.lt_of_not_le hs),
        if_neg (fun hmem => hs ((mem_reachedMin_iff hmin cur k).mp hmem)),
        hcurCnt]
      omega
  · -- `min < max` branch
    rw [if_neg hmm]
    dsimp only
    generalize hrfa : cur.keys.filter
      (fun k' => decide (max ≤ cur.countKey k')) = rfa
    generalize hrm : cur.keys.filter
      (fun k' => decide (min ≤ cur.countKeyP k' Except.isOk)) = rm
    have hrmIff : ∀ k : κ, k ∈ rm ↔ min ≤ cur.countKeyP k Except.isOk := by
      intro k
      rw [← hrm]
      exact mem_reachedMin_iff hmin cur k
    have hmem_mbnm' : ∀ k : κ, k ∈ rm.filterNotIn rfa ↔ (k ∈ rm ∧ k ∉ rfa) := by
      intro k
      unfold Stream.filterNotIn Stream.filter
      rw [List.mem_filter]
      constructor
      · rintro ⟨h1, h2⟩
        refine ⟨h1, fun hc => ?_⟩
        have hcc : rfa.contains k = true := List.elem_eq_true_of_mem hc
        rw [hcc] at h2
        cases h2
      · rintro ⟨h1, h2⟩
        refine ⟨h1, ?_⟩
        show (!rfa.contains k) = true
        rw [Bool.eq_false_iff.mpr fun hcc => h2 (List.contains_iff_mem.mp hcc)]
        rfl
    refine { count := fun k v => ?_, parked := fun k hk => ?_,
             mbnmEmpty := fun heq => absurd heq hmm }
    · -- the count clause
      rw [outCnt_append, okCnt_append,
        outCnt_emit k v _ _ (emit_pointwise k v _ (fun c w => rfl)
          (fun c err => rfl)),
        okCnt_antiJoin]
      by_cases hpk : k ∈ st.minButNotMax
      · -- parked: emits nothing; stays parked or is dropped entirely
        rw [if_pos hpk]
        have holdC := h.count k v
        rw [if_pos hpk] at holdC
        have hstill : min ≤ cur.countKeyP k Except.isOk := by
          have := h.parked k hpk
          rw [hcurKP]
          omega
        by_cases hr : k ∈ rfa
        · rw [if_neg (fun hm => ((hmem_mbnm' k).mp hm).2 hr),
            okCnt_antiJoin, if_pos hr]
          omega
        · rw [if_pos ((hmem_mbnm' k).mpr ⟨(hrmIff k).mpr hstill, hr⟩)]
          omega
      · rw [if_neg hpk]
        have holdC := h.count k v
        rw [if_neg hpk] at holdC
        rw [okCnt_filter_key k v cur
          (fun k' => !decide (cur.countKeyP k' Except.isOk < min))]
        by_cases hs : min ≤ cur.countKeyP k Except.isOk
        · rw [if_pos (by simpa using Nat.not_lt.mpr hs)]
          by_cases hr : k ∈ rfa
          · rw [if_neg (fun hm => ((hmem_mbnm' k).mp hm).2 hr),
              okCnt_antiJoin, if_pos hr, hcurCnt]
            omega
          · rw [if_pos ((hmem_mbnm' k).mpr ⟨(hrmIff k).mpr hs, hr⟩), hcurCnt]
            omega
        · rw [if_neg (by simpa using Nat.lt_of_not_le hs)]
          have hnm : k ∉ rm := fun hm => hs ((hrmIff k).mp hm)
          rw [if_neg (fun hm => hnm ((hmem_mbnm' k).mp hm).1), okCnt_antiJoin]
          by_cases hr : k ∈ rfa
          · rw [if_pos hr]
            omega
          · rw [if_neg hr, hcurCnt]
            omega
    · -- the parked clause for the new state
      obtain ⟨hkrm, hkrfa⟩ := (hmem_mbnm' k).mp hk
      have hs : min ≤ cur.countKeyP k Except.isOk := (hrmIff k).mp hkrm
      rw [countKeyP_antiJoin', if_neg hkrfa]
      exact hs

/-- The accounting invariant along any WR run prefix. -/
theorem wrcs_runFrom {min max : Nat} (hmin : 1 ≤ min)
    (consumed : Stream (κ × Except E V)) (st : QuorumWRState κ V E)
    (out : Stream (κ × V)) (h : WRCS min max consumed st out)
    (ts : List (Stream (κ × Except E V))) :
    WRCS min max (consumed ++ ts.flatten)
      ((collectQuorumWRTick min max).runFrom st ts).1
      (out ++ allTicks ((collectQuorumWRTick min max).runFrom st ts).2) := by
  induction ts generalizing consumed st out with
  | nil =>
    simpa [allTicks] using h
  | cons b bs ih =>
    rw [TickLoop.runFrom_cons]
    have hstep := wrcs_step (b := b) hmin h
    have := ih (consumed ++ b) _ _ hstep
    rw [List.flatten_cons, ← List.append_assoc]
    show WRCS min max ((consumed ++ b) ++ bs.flatten) _ _
    have harr : out ++
        allTicks (((collectQuorumWRTick min max).step st b).2
          :: ((collectQuorumWRTick min max).runFrom
            ((collectQuorumWRTick min max).step st b).1 bs).2)
      = (out ++ ((collectQuorumWRTick min max).step st b).2) ++
          allTicks (((collectQuorumWRTick min max).runFrom
            ((collectQuorumWRTick min max).step st b).1 bs).2) := by
      show out ++ (((collectQuorumWRTick min max).step st b).2 ++ _) = _
      rw [List.append_assoc]
      rfl
    rw [harr]
    exact this

/-- **Per-(key, value) count soundness for `collect_quorum_with_response`**
(unconditional — no usage contract): across every batching, the number of
`(k, v)` pairs emitted never exceeds the number of `(k, .ok v)` responses
consumed. With per-acceptor at-most-one-`Ok`-per-ballot (the guarded
variant), this converts collected-payload multiplicity into **distinct
responders** — the fact whose absence is FINDINGS.md B1. -/
theorem wrRun_outCnt_le {min max : Nat} (hmin : 1 ≤ min)
    (ts : List (Stream (κ × Except E V))) (k : κ) (v : V) :
    outCnt k v (allTicks ((collectQuorumWRTick min max).run ts).2)
      ≤ okCnt k v ts.flatten := by
  have h := wrcs_runFrom hmin [] ⟨[], []⟩ [] (wrcs_init min max) ts
  have hc := h.count k v
  show outCnt k v ([] ++ allTicks ((collectQuorumWRTick min max).runFrom
      (⟨[], []⟩ : QuorumWRState κ V E) ts).2) ≤ okCnt k v ([] ++ ts.flatten)
  rcases Decidable.em (k ∈ ((collectQuorumWRTick min max).runFrom
      (⟨[], []⟩ : QuorumWRState κ V E) ts).1.minButNotMax) with hp | hp
  · rw [if_pos hp] at hc
    rw [Nat.add_zero] at hc
    exact hc
  · rw [if_neg hp] at hc
    exact Nat.le_trans (Nat.le_add_right _ _) hc


/-! ## Step-level membership transport for the WR loop -/

omit [DecidableEq V] in
/-- Retained responses after one WR tick were current responses. -/
theorem wrStep_notAll_mem_chain {min max : Nat}
    {st : QuorumWRState κ V E} {b : Stream (κ × Except E V)}
    {e : κ × Except E V}
    (h : e ∈ ((collectQuorumWRTick min max).step st b).1.notAll) :
    e ∈ st.notAll.chain b := by
  simp only [collectQuorumWRTick] at h
  by_cases hmm : min = max
  · rw [if_pos hmm] at h
    exact Stream.mem_of_mem_antiJoin h
  · rw [if_neg hmm] at h
    exact Stream.mem_of_mem_antiJoin h

omit [DecidableEq V] in
/-- Emissions of one WR tick are `Ok` current responses. -/
theorem wrStep_emit_mem_chain {min max : Nat}
    {st : QuorumWRState κ V E} {b : Stream (κ × Except E V)}
    {k : κ} {v : V}
    (h : (k, v) ∈ ((collectQuorumWRTick min max).step st b).2) :
    (k, .ok v) ∈ st.notAll.chain b := by
  simp only [collectQuorumWRTick] at h
  by_cases hmm : min = max
  · rw [if_pos hmm] at h
    dsimp only at h
    have := wr_emit_mem (by
      -- normalize the emit lambda to the `wr_emit_mem` shape
      exact h)
    exact (List.mem_filter.mp this).1
  · rw [if_neg hmm] at h
    dsimp only at h
    have := wr_emit_mem (by exact h)
    exact (List.mem_filter.mp (Stream.mem_of_mem_antiJoin this)).1

end HydroLean.Programs
