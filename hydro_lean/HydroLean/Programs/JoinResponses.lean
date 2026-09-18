import HydroLean.Hydro.Join
import HydroLean.Hydro.TStream
import HydroLean.Hydro.Growth

/-!
# `join_responses`: request–response metadata joining

Port of `join_responses` from `hydro_std/src/request_response.rs`. The Rust
code is a `sliced!` block:

```rust
let mut remaining_to_join = use::state_null::<Stream<(K, M), _, _, NoOrder>>();
let response_batch = use::batch(responses, nondet!(
    /// Because we persist the metadata, delays resulting from
    /// batching boundaries do not affect the output contents.
));
let metadata_batch = use::atomic(metadata.all_ticks_atomic(), nondet!(...));

let remaining_and_new = remaining_to_join.chain(metadata_batch);
let joined_this_tick = remaining_and_new.clone().join(response_batch.clone())
    .map(q!(|(key, (meta, resp))| (key, (meta, resp))));
remaining_to_join = remaining_and_new.anti_join(response_batch.map(q!(|(key, _)| key)));
joined_this_tick
```

The `nondet!` guard on `use::batch(responses)` says: *"Because we persist the
metadata, delays resulting from batching boundaries do not affect the output
contents."* Here that English claim becomes the formal shape of the theorem:
we quantify over **all** tick-aligned input sequences (each tick's metadata
and response batches — the batching *is* the adversarial choice), and prove
the flattened output is always the full metadata⋈response join, each pair
exactly once.

The Rust doc contract becomes explicit hypotheses:
- *"Only one response element should be produced with a given key, same for
  the metadata stream"* → `Nodup` of the flattened key lists;
- *"The metadata must be generated in the same or a previous tick than the
  response"* → the `Causal` predicate below.
-/

namespace HydroLean.Programs

open HydroLean.Hydro

universe u v w

/-- Per-tick input of `join_responses`: this tick's metadata batch (Rust:
`metadata_batch`) and response batch (Rust: `response_batch`; its boundaries
are the `nondet!`-guarded adversarial choice). -/
structure JoinTickIn (K : Type u) (M : Type v) (V : Type w) : Type (max u v w) where
  metadata : List (K × M)
  responses : List (K × V)

variable {K : Type u} {M : Type v} {V : Type w} [DecidableEq K]

/-- The `join_responses` tick loop. Looped state is `remaining_to_join`
(metadata not yet matched by a response); the body mirrors the Rust
statement-for-statement. -/
def joinResponsesTick (K : Type u) (M : Type v) (V : Type w) [DecidableEq K] :
    TickLoop (JoinTickIn K M V) (List (K × M)) (List (K × (M × V))) where
  init := []
  step := fun remaining_to_join input =>
    let remaining_and_new := Stream.chain remaining_to_join input.metadata
    let joined_this_tick := Stream.join remaining_and_new input.responses
    (Stream.antiJoin remaining_and_new (input.responses.map Prod.fst),
      joined_this_tick)

/-- All metadata delivered across a run. -/
def allMeta (ts : List (JoinTickIn K M V)) : List (K × M) :=
  (ts.map (·.metadata)).flatten

/-- All responses delivered across a run. -/
def allResp (ts : List (JoinTickIn K M V)) : List (K × V) :=
  (ts.map (·.responses)).flatten

omit [DecidableEq K] in
@[simp] theorem allMeta_nil : allMeta ([] : List (JoinTickIn K M V)) = [] := rfl
omit [DecidableEq K] in
@[simp] theorem allResp_nil : allResp ([] : List (JoinTickIn K M V)) = [] := rfl
omit [DecidableEq K] in
@[simp] theorem allMeta_cons (t : JoinTickIn K M V) (ts : List (JoinTickIn K M V)) :
    allMeta (t :: ts) = t.metadata ++ allMeta ts := rfl
omit [DecidableEq K] in
@[simp] theorem allResp_cons (t : JoinTickIn K M V) (ts : List (JoinTickIn K M V)) :
    allResp (t :: ts) = t.responses ++ allResp ts := rfl

/-- The causality contract (Rust doc: *"The metadata must be generated in the
same or a previous tick than the response"*): every response key has already
appeared among the metadata keys seen up to and including its tick. `seen` is
the accumulator of metadata keys from previous ticks. -/
def Causal (seen : List K) : List (JoinTickIn K M V) → Prop
  | [] => True
  | t :: ts =>
    (∀ q ∈ t.responses, q.1 ∈ seen ++ t.metadata.map Prod.fst) ∧
      Causal (seen ++ t.metadata.map Prod.fst) ts

/-! ## List facts used by the invariant -/

omit [DecidableEq K] in
private theorem nodup_of_append_left {l₁ l₂ : List K} (h : (l₁ ++ l₂).Nodup) :
    l₁.Nodup := (List.pairwise_append.mp h).1

omit [DecidableEq K] in
private theorem nodup_of_append_right {l₁ l₂ : List K} (h : (l₁ ++ l₂).Nodup) :
    l₂.Nodup := (List.pairwise_append.mp h).2.1

omit [DecidableEq K] in
private theorem disj_of_append {l₁ l₂ : List K} (h : (l₁ ++ l₂).Nodup) :
    ∀ a ∈ l₁, ∀ b ∈ l₂, a ≠ b := (List.pairwise_append.mp h).2.2

omit [DecidableEq K] in
/-! ## The run invariant -/

/-- The looped state as a function of the consumed prefix: the metadata whose
key has not yet been answered. -/
def pendingState (M₁ : List (K × M)) (R₁ : List (K × V)) : List (K × M) :=
  M₁.filter (fun q => !(R₁.map Prod.fst).contains q.1)

@[simp] theorem pendingState_nil_resp (M₁ : List (K × M)) :
    pendingState M₁ ([] : List (K × V)) = M₁ := by
  simp [pendingState]

/-- One-step state preservation: after a tick with batches `(Mt, Rt)` from the
pending state for `(M₁, R₁)`, the new state is the pending state for
`(M₁ ++ Mt, R₁ ++ Rt)` — **provided** no key of `Mt` was already answered in
`R₁` (which the causality + nodup contract guarantees). -/
theorem pendingState_step (M₁ : List (K × M)) (R₁ : List (K × V))
    (Mt : List (K × M)) (Rt : List (K × V))
    (hfresh : ∀ k ∈ Mt.map Prod.fst, k ∉ R₁.map Prod.fst) :
    Stream.antiJoin (pendingState M₁ R₁ ++ Mt) (Rt.map Prod.fst)
      = pendingState (M₁ ++ Mt) (R₁ ++ Rt) := by
  simp only [Stream.antiJoin, Stream.filter, pendingState, List.filter_append,
    List.filter_filter]
  congr 1
  · apply List.filter_congr
    intro q _
    simp only [List.map_append, List.contains_append]
    cases h₁ : (R₁.map Prod.fst).contains q.1 <;>
      cases h₂ : (Rt.map Prod.fst).contains q.1 <;> simp_all
  · apply List.filter_congr
    intro q hq
    have hnot : (R₁.map Prod.fst).contains q.1 = false := by
      have : q.1 ∉ R₁.map Prod.fst :=
        hfresh q.1 (List.mem_map.mpr ⟨q, hq, rfl⟩)
      simpa using this
    simp only [List.map_append, List.contains_append, hnot]
    cases h₂ : (Rt.map Prod.fst).contains q.1 <;> simp_all

/-- **The run invariant** (generalized over an already-consumed prefix
`(M₁, R₁)`): running `join_responses` from the pending state emits, across all
ticks, exactly one joined pair per answered key. -/
theorem joinResponses_invariant (M₁ : List (K × M)) (R₁ : List (K × V))
    (ts : List (JoinTickIn K M V))
    (hm : ((M₁ ++ allMeta ts).map Prod.fst).Nodup)
    (hr : ((R₁ ++ allResp ts).map Prod.fst).Nodup)
    (hR₁ : ∀ k ∈ R₁.map Prod.fst, k ∈ M₁.map Prod.fst)
    (hc : Causal (M₁.map Prod.fst) ts) :
    ((allTicks ((joinResponsesTick K M V).runFrom (pendingState M₁ R₁) ts).2).map
        Prod.fst).Nodup ∧
    ∀ k (m : M) (v : V),
      (k, (m, v)) ∈ allTicks ((joinResponsesTick K M V).runFrom (pendingState M₁ R₁) ts).2
        ↔ ((k, m) ∈ M₁ ++ allMeta ts ∧ (k, v) ∈ allResp ts) := by
  induction ts generalizing M₁ R₁ with
  | nil =>
    refine ⟨by simp [allTicks], ?_⟩
    intro k m v
    simp [allTicks]
  | cons t ts ih =>
    -- abbreviations
    obtain ⟨hct, hc'⟩ := hc
    -- flattened-key nodup hypotheses, regrouped
    have hmC : ((M₁ ++ (t.metadata ++ allMeta ts)).map Prod.fst).Nodup := by
      rw [allMeta_cons] at hm; exact hm
    have hrC : ((R₁ ++ (t.responses ++ allResp ts)).map Prod.fst).Nodup := by
      rw [allResp_cons] at hr; exact hr
    have hmKeys : (M₁.map Prod.fst ++ (t.metadata.map Prod.fst
        ++ (allMeta ts).map Prod.fst)).Nodup := by
      rw [List.map_append, List.map_append] at hmC; exact hmC
    have hrKeys : (R₁.map Prod.fst ++ (t.responses.map Prod.fst
        ++ (allResp ts).map Prod.fst)).Nodup := by
      rw [List.map_append, List.map_append] at hrC; exact hrC
    -- left-grouped variant, for splitting off the tail
    have hmKeysL : ((M₁.map Prod.fst ++ t.metadata.map Prod.fst)
        ++ (allMeta ts).map Prod.fst).Nodup := by
      rw [List.append_assoc]; exact hmKeys
    -- freshness of this tick's metadata keys w.r.t. already-consumed responses
    have hfresh : ∀ k ∈ t.metadata.map Prod.fst, k ∉ R₁.map Prod.fst := by
      intro k hkMt hkR₁
      have hkM₁ : k ∈ M₁.map Prod.fst := hR₁ k hkR₁
      exact disj_of_append hmKeys k hkM₁ k (List.mem_append.mpr (.inl hkMt)) rfl
    -- the step of the tick loop
    have hstep : (joinResponsesTick K M V).step (pendingState M₁ R₁) t
        = (pendingState (M₁ ++ t.metadata) (R₁ ++ t.responses),
            Stream.join (pendingState M₁ R₁ ++ t.metadata) t.responses) := by
      simp only [joinResponsesTick, Stream.chain]
      rw [pendingState_step M₁ R₁ t.metadata t.responses hfresh]
    -- instantiate the IH at the extended prefix
    have hm' : (((M₁ ++ t.metadata) ++ allMeta ts).map Prod.fst).Nodup := by
      have := hmC; rw [← List.append_assoc] at this; exact this
    have hr' : (((R₁ ++ t.responses) ++ allResp ts).map Prod.fst).Nodup := by
      have := hrC; rw [← List.append_assoc] at this; exact this
    have hR₁' : ∀ k ∈ (R₁ ++ t.responses).map Prod.fst,
        k ∈ (M₁ ++ t.metadata).map Prod.fst := by
      intro k hk
      rw [List.map_append] at hk ⊢
      cases List.mem_append.mp hk with
      | inl h => exact List.mem_append.mpr (.inl (hR₁ k h))
      | inr h =>
        obtain ⟨q, hq, rfl⟩ := List.mem_map.mp h
        exact hct q hq
    have hc'' : Causal ((M₁ ++ t.metadata).map Prod.fst) ts := by
      rw [List.map_append]; exact hc'
    have ⟨ihNodup, ihMem⟩ := ih (M₁ ++ t.metadata) (R₁ ++ t.responses) hm' hr' hR₁' hc''
    -- unfold one tick of the run
    rw [TickLoop.runFrom_cons, hstep]
    simp only [allTicks, allMeta_cons, allResp_cons, List.flatten_cons]
      at ihNodup ihMem ⊢
    constructor
    -- (1) Nodup of emitted keys
    · rw [List.map_append, List.nodup_append]
      have houtKeys : ∀ k ∈ (Stream.join (pendingState M₁ R₁ ++ t.metadata)
          t.responses).map Prod.fst, k ∈ t.responses.map Prod.fst :=
        fun k hk => Stream.mem_keys_join_right hk
      refine ⟨?_, ihNodup, ?_⟩
      · -- keys of this tick's join output are nodup
        apply Stream.keys_join_nodup
        · -- left keys nodup: sublist of (M₁ ++ Mt) keys
          have hsub : ((pendingState M₁ R₁ ++ t.metadata).map Prod.fst).Sublist
              ((M₁ ++ t.metadata).map Prod.fst) := by
            apply List.Sublist.map
            exact List.Sublist.append_right List.filter_sublist t.metadata
          refine hsub.nodup ?_
          rw [List.map_append]
          exact nodup_of_append_left hmKeysL
        · -- right keys nodup: middle segment of hrKeys
          exact nodup_of_append_left
            (nodup_of_append_right (l₁ := R₁.map Prod.fst) hrKeys)
      · -- disjointness with later ticks' keys
        intro a ha b hb
        have haR : a ∈ t.responses.map Prod.fst := houtKeys a ha
        -- keys of the rest of the run come from later responses
        have hbR : b ∈ (allResp ts).map Prod.fst := by
          obtain ⟨q, hq, rfl⟩ := List.mem_map.mp hb
          obtain ⟨k', m', v'⟩ := q
          exact List.mem_map.mpr ⟨(k', v'), ((ihMem k' m' v').mp hq).2, rfl⟩
        exact disj_of_append
          (nodup_of_append_right (l₁ := R₁.map Prod.fst) hrKeys) a haR b hbR
    -- (2) membership characterization
    · intro k m v
      rw [List.mem_append, ihMem k m v]
      constructor
      · intro h
        cases h with
        | inl h =>
          have ⟨hl, hrr⟩ := Stream.mem_join.mp h
          refine ⟨?_, List.mem_append.mpr (.inl hrr)⟩
          cases List.mem_append.mp hl with
          | inl h' =>
            exact List.mem_append.mpr (.inl (List.mem_filter.mp h').1)
          | inr h' =>
            exact List.mem_append.mpr (.inr (List.mem_append.mpr (.inl h')))
        | inr h =>
          refine ⟨?_, List.mem_append.mpr (.inr h.2)⟩
          have := h.1
          simpa [List.append_assoc] using this
      · intro ⟨hkm, hkv⟩
        cases List.mem_append.mp hkv with
        | inr hR₂ =>
          -- answered later: the pair is emitted by a later tick (IH)
          refine .inr ⟨?_, hR₂⟩
          simpa [List.append_assoc] using hkm
        | inl hRt =>
          -- answered this tick: the metadata must be pending or fresh
          refine .inl (Stream.mem_join.mpr ⟨?_, hRt⟩)
          -- k's metadata cannot be in a later tick (causality + nodup)
          have hkSeen : k ∈ M₁.map Prod.fst ++ t.metadata.map Prod.fst :=
            hct (k, v) hRt
          have hkmEarly : (k, m) ∈ M₁ ++ t.metadata := by
            cases List.mem_append.mp hkm with
            | inl h => exact List.mem_append.mpr (.inl h)
            | inr h =>
              cases List.mem_append.mp h with
              | inl h' => exact List.mem_append.mpr (.inr h')
              | inr h' =>
                -- (k, m) ∈ allMeta ts contradicts k ∈ seen keys
                exact absurd rfl (disj_of_append hmKeysL
                  k hkSeen
                  k (List.mem_map.mpr ⟨(k, m), h', rfl⟩))
          cases List.mem_append.mp hkmEarly with
          | inr h' => exact List.mem_append.mpr (.inr h')
          | inl h' =>
            -- pending: k not answered in R₁ (response keys nodup)
            refine List.mem_append.mpr (.inl ?_)
            rw [pendingState, List.mem_filter]
            refine ⟨h', ?_⟩
            have hkRt : k ∈ t.responses.map Prod.fst :=
              List.mem_map.mpr ⟨(k, v), hRt, rfl⟩
            have : k ∉ R₁.map Prod.fst := fun hkR₁ =>
              disj_of_append hrKeys k hkR₁ k (List.mem_append.mpr (.inl hkRt)) rfl
            simpa using this

/-- **Theorem (`join_responses` correctness, unbounded)**: for every
tick-aligned input sequence satisfying the request–response contract
(duplicate-free keys on each side; responses never precede their metadata),
the `all_ticks` output contains exactly the pairs `(k, (m, v))` with `(k, m)`
in the metadata stream and `(k, v)` in the response stream — **each key
emitted exactly once**, regardless of how the runtime chopped the streams into
ticks.

This discharges, in unbounded form, the Rust `nondet!` justification
*"Because we persist the metadata, delays resulting from batching boundaries
do not affect the output contents."* -/
theorem joinResponses_correct (ts : List (JoinTickIn K M V))
    (hm : ((allMeta ts).map Prod.fst).Nodup)
    (hr : ((allResp ts).map Prod.fst).Nodup)
    (hc : Causal [] ts) :
    ((allTicks ((joinResponsesTick K M V).run ts).2).map Prod.fst).Nodup ∧
    ∀ k (m : M) (v : V),
      (k, (m, v)) ∈ allTicks ((joinResponsesTick K M V).run ts).2
        ↔ ((k, m) ∈ allMeta ts ∧ (k, v) ∈ allResp ts) := by
  have := joinResponses_invariant ([] : List (K × M)) ([] : List (K × V)) ts
    (by simpa using hm) (by simpa using hr) (by simp) (by simpa using hc)
  simpa [TickLoop.run, joinResponsesTick, pendingState] using this

/-- **Unconditional emission provenance**: every joined output quotes a
delivered metadata entry and a delivered response — `join_responses` never
fabricates, even off-contract (no key-nodup or causality hypotheses). -/
theorem joinResponses_emit_mem {ts : List (JoinTickIn K M V)} {k : K}
    {m : M} {v : V}
    (h : (k, (m, v)) ∈ allTicks ((joinResponsesTick K M V).run ts).2) :
    (k, m) ∈ allMeta ts ∧ (k, v) ∈ allResp ts := by
  suffices hgen : ∀ (ts : List (JoinTickIn K M V)) (s past : List (K × M)),
      (∀ x ∈ s, x ∈ past) →
      (k, (m, v)) ∈ allTicks ((joinResponsesTick K M V).runFrom s ts).2 →
      (k, m) ∈ past ++ allMeta ts ∧ (k, v) ∈ allResp ts by
    have := hgen ts [] [] (fun _ hx => nomatch hx) h
    simpa using this
  clear h
  intro ts
  induction ts with
  | nil =>
    intro s past _ h
    cases h
  | cons t rest ih =>
    intro s past hsub h
    rw [TickLoop.runFrom_cons] at h
    rcases List.mem_append.mp (h : (k, (m, v)) ∈ _ ++ allTicks _) with
      hhead | htail
    · -- joined this tick
      have hj := Stream.mem_join.mp hhead
      constructor
      · rcases List.mem_append.mp (hj.1 : (k, m) ∈ s ++ t.metadata) with
          hs | hm
        · exact List.mem_append_left _ (hsub _ hs)
        · rw [allMeta_cons, ← List.append_assoc]
          exact List.mem_append_left _ (List.mem_append_right _ hm)
      · rw [allResp_cons]
        exact List.mem_append_left _ hj.2
    · -- joined later; the looped state stays within past metadata
      have hstate : ∀ x ∈ ((joinResponsesTick K M V).step s t).1,
          x ∈ past ++ t.metadata := by
        intro x hx
        have hx' : x ∈ Stream.chain s t.metadata :=
          (List.mem_filter.mp hx).1
        rcases List.mem_append.mp hx' with hs | hm
        · exact List.mem_append_left _ (hsub _ hs)
        · exact List.mem_append_right _ hm
      have := ih _ (past ++ t.metadata) hstate htail
      refine ⟨?_, ?_⟩
      · rw [allMeta_cons, ← List.append_assoc]
        exact this.1
      · rw [allResp_cons]
        exact List.mem_append_right _ this.2

/-! ## Executable mirror of the Rust sim test

`test_join_responses_basic`: metadata `(1, 42)` in an earlier tick, response
`(1, "hello")` later — emits `(1, (42, "hello"))`. -/

#guard allTicks ((joinResponsesTick Nat Nat String).run
    [⟨[(1, 42)], []⟩, ⟨[], [(1, "hello")]⟩]).2
    = [(1, (42, "hello"))]
-- same tick delivery also joins
#guard allTicks ((joinResponsesTick Nat Nat String).run
    [⟨[(1, 42)], [(1, "hello")]⟩]).2
    = [(1, (42, "hello"))]
-- unanswered metadata emits nothing; unrelated keys don't join
#guard allTicks ((joinResponsesTick Nat Nat String).run
    [⟨[(1, 42), (2, 7)], []⟩, ⟨[], [(2, "bye")]⟩]).2
    = [(2, (7, "bye"))]

/-! ## The typed stage and its verified face

`join_responses` over the located surface: both inputs are `NoOrder`
streams, so each is consumed through a `batchC` decision (the consumed batch
IS the decision, shuffle included) — `dmeta` for the `use::atomic` metadata
side, `dresp` for the `use::batch(responses, nondet!(…))` side. The stage is
the single source of the dataflow; the face below states its input/output
contract over the run. -/

section Stage

variable {K M V : Type} [DecidableEq K] [DecidableEq M] [DecidableEq V]

open HydroLean.Hydro in
/-- `join_responses` as the typed stage (request_response.rs). -/
def join_responsesM (dmeta : List (List (K × M)))
    (dresp : List (List (K × V))) :
    Stream (K × M) × Stream (K × V) →ₘ TStream (K × (M × V)) :=
  let metadata_batch :=
    (MonoMap.fst (α := Stream (K × M)) (β := Stream (K × V))).asCnt.batchC dmeta
  let response_batch :=
    (MonoMap.snd (α := Stream (K × M)) (β := Stream (K × V))).asCnt.batchC dresp
  ((metadata_batch.zipWith response_batch
    (fun ms rs => JoinTickIn.mk ms rs)).loop (joinResponsesTick K M V))

open HydroLean.Hydro in
/-- The run reduces to the tick loop over the zipped consumed batches
(definitional bridge; under complete decisions `batchC` realizes the
decisions themselves, `Consumes.batchC_eq`). -/
theorem join_responsesM_run (dmeta : List (List (K × M)))
    (dresp : List (List (K × V))) (md : Stream (K × M))
    (resp : Stream (K × V))
    (hm : Consumes md dmeta) (hr : Consumes resp dresp) :
    (join_responsesM dmeta dresp).f (md, resp)
      = (joinResponsesTick K M V).outputs
          (List.zipWith (fun ms rs => JoinTickIn.mk ms rs) dmeta dresp) := by
  show (joinResponsesTick K M V).outputs
      (List.zipWith _ (batchC md [] dmeta) (batchC resp [] dresp)) = _
  rw [hm.batchC_eq, hr.batchC_eq]

omit [DecidableEq K] [DecidableEq M] [DecidableEq V] in
private theorem allMeta_zip {dmeta : List (List (K × M))}
    {dresp : List (List (K × V))} (hlen : dmeta.length = dresp.length) :
    allMeta (List.zipWith (fun ms rs => JoinTickIn.mk ms rs) dmeta dresp)
      = dmeta.flatten := by
  induction dmeta generalizing dresp with
  | nil => rfl
  | cons m dm ih =>
    cases dresp with
    | nil => exact absurd hlen (by simp)
    | cons r dr =>
      rw [List.zipWith_cons_cons, allMeta_cons, List.flatten_cons,
        ih (by simpa using hlen)]

omit [DecidableEq K] [DecidableEq M] [DecidableEq V] in
private theorem allResp_zip {dmeta : List (List (K × M))}
    {dresp : List (List (K × V))} (hlen : dmeta.length = dresp.length) :
    allResp (List.zipWith (fun ms rs => JoinTickIn.mk ms rs) dmeta dresp)
      = dresp.flatten := by
  induction dmeta generalizing dresp with
  | nil => cases dresp with
    | nil => rfl
    | cons r dr => exact absurd hlen (by simp)
  | cons m dm ih =>
    cases dresp with
    | nil => exact absurd hlen (by simp)
    | cons r dr =>
      rw [List.zipWith_cons_cons, allResp_cons, List.flatten_cons,
        ih (by simpa using hlen)]

open HydroLean.Hydro in
/-- **Verified face of `join_responses`**: inputs — complete consumption
decisions on both `NoOrder` sides (equal tick counts: the two `use` sites
share the `sliced!` clock), the Rust doc contract *"only one response
element should be produced with a given key, same for the metadata stream"*
(key-nodup on both raw streams), and causality (*"the metadata must be
generated in the same or a previous tick than the response"*, over the
decided ticks). Output — the joined stream contains exactly
metadata ⋈ responses, **each key exactly once**, for every decision pair.

This discharges, in unbounded form, the Rust `nondet!` justification
*"Because we persist the metadata, delays resulting from batching boundaries
do not affect the output contents."* -/
theorem join_responses_spec (dmeta : List (List (K × M)))
    (dresp : List (List (K × V))) (md : Stream (K × M))
    (resp : Stream (K × V))
    (hmc : Consumes md dmeta) (hrc : Consumes resp dresp)
    (hlen : dmeta.length = dresp.length)
    (hm : (md.map Prod.fst).Nodup) (hr : (resp.map Prod.fst).Nodup)
    (hc : Causal []
      (List.zipWith (fun ms rs => JoinTickIn.mk ms rs) dmeta dresp)) :
    ((allTicks ((join_responsesM dmeta dresp).f (md, resp))).map
      Prod.fst).Nodup ∧
    ∀ k (m : M) (v : V),
      (k, (m, v)) ∈ allTicks ((join_responsesM dmeta dresp).f (md, resp))
        ↔ ((k, m) ∈ md ∧ (k, v) ∈ resp) := by
  rw [join_responsesM_run dmeta dresp md resp hmc hrc,
    show (joinResponsesTick K M V).outputs
        (List.zipWith (fun ms rs => JoinTickIn.mk ms rs) dmeta dresp)
      = ((joinResponsesTick K M V).run
          (List.zipWith (fun ms rs => JoinTickIn.mk ms rs) dmeta dresp)).2
      from rfl]
  have hm' : ((allMeta (List.zipWith (fun ms rs => JoinTickIn.mk ms rs)
      dmeta dresp)).map Prod.fst).Nodup := by
    rw [allMeta_zip hlen]
    exact (hmc.map Prod.fst).nodup_iff.mpr hm
  have hr' : ((allResp (List.zipWith (fun ms rs => JoinTickIn.mk ms rs)
      dmeta dresp)).map Prod.fst).Nodup := by
    rw [allResp_zip hlen]
    exact (hrc.map Prod.fst).nodup_iff.mpr hr
  obtain ⟨hnodup, hmem⟩ := joinResponses_correct _ hm' hr' hc
  refine ⟨hnodup, fun k m v => ?_⟩
  rw [hmem k m v, allMeta_zip hlen, allResp_zip hlen]
  exact and_congr hmc.mem_iff hrc.mem_iff

end Stage

/-- **Zip-call elimination (module face)**: at the standard call shape —
tick-aligned metadata zipped with tick-aligned responses — a joined
emission quotes a delivered metadata entry and a delivered response
(unconditional; the tick-aligned form of `joinResponses_emit_mem`). -/
theorem joinResponses_zip_elim {metaT : TStream (K × M)}
    {respT : TStream (K × V)} {k : K} {m : M} {v : V}
    (h : (k, (m, v)) ∈ (scan (joinResponsesTick K M V).step []
      ((TSing.zip metaT respT).map
        (fun mr => JoinTickIn.mk mr.1 mr.2))).flatten) :
    (k, m) ∈ metaT.flatten ∧ (k, v) ∈ respT.flatten := by
  have hout : (k, (m, v)) ∈ TStream.allTicks
      ((joinResponsesTick K M V).outputs
        ((TSing.zip metaT respT).map (fun mr => JoinTickIn.mk mr.1 mr.2))) := by
    rw [← scan_eq_outputs]
    exact h
  have hjoin := joinResponses_emit_mem hout
  constructor
  · have hsub : (((TSing.zip metaT respT).map
        (fun mr => JoinTickIn.mk mr.1 mr.2)).map (·.metadata)) <+: metaT := by
      rw [show (((TSing.zip metaT respT).map
          (fun mr => JoinTickIn.mk mr.1 mr.2)).map (·.metadata))
          = (List.zip metaT respT).map Prod.fst from by
        rw [List.map_map]
        rfl]
      exact zip_fst_prefix _ _
    exact (prefix_flatten hsub).subset hjoin.1
  · have hsub : (((TSing.zip metaT respT).map
        (fun mr => JoinTickIn.mk mr.1 mr.2)).map (·.responses)) <+: respT := by
      rw [show (((TSing.zip metaT respT).map
          (fun mr => JoinTickIn.mk mr.1 mr.2)).map (·.responses))
          = (List.zip metaT respT).map Prod.snd from by
        rw [List.map_map]
        rfl]
      exact zip_snd_prefix _ _
    exact (prefix_flatten hsub).subset hjoin.2

end HydroLean.Programs
