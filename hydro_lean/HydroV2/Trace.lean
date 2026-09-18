import HydroV2.Grades

/-!
# HydroV2 · tick traces and cut legality

The realized-run vocabulary the interpretations share:

- `Trace σ` — a tick-located value's realized values **across all
  ticks** (one per realized tick); a batch stream is simply
  `Trace (List α)`.
- `foldAcrossTicksTrace`/`scanAcrossTicksTrace` — the cross-tick state fold (Rust `use::state`)
  and its emitting form (state + per-tick emission).
- `MonoTrace vo` — a trace bundled with trajectory ascent in `vo`: the
  realized form of Rust's `Monotonic` singleton bound.
- `BatchLegal`/`batchCuts`/`snapshotCuts` — decisions-as-inputs for the
  consumption points: a `batch`/`snapshot` decision is the consumed
  increment itself, legal iff it fits the pool's multiset (an illegal
  increment blocks — legality is realizability).
- `iterate` — guarded cycles (Rust `forward_ref`) as Kleene iteration.


## Naming: pure carriers ↔ `HydroSem` operators ↔ Rust

Each pure function here is the `Values` denotation of a Sem operator —
the operator's camelCase plus `Trace` (the realized run it computes),
and the Sem operator carries the Rust API name:

| Rust construct | Sem operator | pure carrier |
|---|---|---|
| `use::state` register (fold) | `fold_across_ticks_monotone` | `foldAcrossTicks(Monotone)Trace` |
| `use::state` loop (emitting) | `scan_across_ticks` | `scanAcrossTicksTrace` |
| `.batch(&tick, nondet!)` | `batch` | `batchCuts` |
| `.snapshot(&tick, nondet!)` | `snapshot` | `prefixCuts`/`snapshotCuts` |
| `.assume_ordering(nondet!)` | `assume_ordering` | `selectOrder` |
| `forward_ref` cycle | `fix_stream`/`fix_tick` | `iterate` |
-/

namespace HydroV2

/-- A tick-scoped value's realized trace. -/
abbrev Trace (σ : Type _) : Type _ := List σ

/-- Same-tick pairing (tick atomicity: both wires read in one tick;
truncates to the jointly realized ticks). -/
def Trace.zip {α β : Type _} (a : Trace α) (b : Trace β) :
    Trace (α × β) :=
  List.zip a b

/-- Flattening preserves prefixes of emission traces. -/
theorem prefix_flatten {α : Type _} {a b : List (List α)}
    (h : a <+: b) : a.flatten <+: b.flatten := by
  obtain ⟨e, rfl⟩ := h
  exact ⟨e.flatten, (List.flatten_append (L₁ := a) (L₂ := e)).symm⟩

/-- Prefixes lift through `filterMap`. -/
theorem prefix_filterMap {α β : Type _} (f : α → Option β)
    {a b : List α} (h : a <+: b) :
    a.filterMap f <+: b.filterMap f := by
  obtain ⟨e, rfl⟩ := h
  exact ⟨e.filterMap f, (List.filterMap_append).symm⟩

/-- Zips of prefixes are prefixes. -/
theorem zip_prefix {α β : Type _} {a a' : List α} {b b' : List β}
    (ha : a <+: a') (hb : b <+: b') :
    Trace.zip a b <+: Trace.zip a' b' := by
  induction a generalizing a' b b' with
  | nil => exact List.nil_prefix
  | cons x xs ih =>
    obtain ⟨u, rfl⟩ := ha
    cases b with
    | nil => exact List.nil_prefix
    | cons y ys =>
      obtain ⟨v, rfl⟩ := hb
      show (x, y) :: Trace.zip xs ys <+: (x, y) :: Trace.zip (xs ++ u) (ys ++ v)
      exact List.cons_prefix_cons.mpr
        ⟨rfl, ih (List.prefix_append _ _) (List.prefix_append _ _)⟩

/-! ## Cross-tick state -/

/-- The state trace of a cross-tick fold (Rust `use::state`): the value
*after* each tick. -/
def foldAcrossTicksTrace {ι σ : Type _} (g : σ → ι → σ) : σ → List ι → Trace σ
  | _, [] => []
  | s, x :: xs => g s x :: foldAcrossTicksTrace g (g s x) xs

@[simp] theorem foldAcrossTicksTrace_length {ι σ : Type _} (g : σ → ι → σ)
    (s : σ) (xs : List ι) : (foldAcrossTicksTrace g s xs).length = xs.length := by
  induction xs generalizing s with
  | nil => rfl
  | cons x rest ih => exact congrArg (· + 1) (ih _)

theorem foldAcrossTicksTrace_getElem {ι σ : Type _} (g : σ → ι → σ) (s : σ)
    (xs : List ι) (t : Nat) (ht : t < (foldAcrossTicksTrace g s xs).length) :
    (foldAcrossTicksTrace g s xs)[t] = (xs.take (t + 1)).foldl g s := by
  induction xs generalizing s t with
  | nil => cases ht
  | cons x rest ih =>
    cases t with
    | zero => rfl
    | succ n =>
      have hn : n < (foldAcrossTicksTrace g (g s x) rest).length := by
        have := ht
        simp only [foldAcrossTicksTrace, List.length_cons] at this
        omega
      show (foldAcrossTicksTrace g (g s x) rest)[n]'hn = _
      rw [ih (g s x) n hn]
      rfl

theorem foldAcrossTicksTrace_prefix {ι σ : Type _} (g : σ → ι → σ) (s : σ)
    {xs ys : List ι} (h : xs <+: ys) :
    foldAcrossTicksTrace g s xs <+: foldAcrossTicksTrace g s ys := by
  obtain ⟨e, rfl⟩ := h
  induction xs generalizing s with
  | nil => exact List.nil_prefix
  | cons x rest ih =>
    exact List.cons_prefix_cons.mpr ⟨rfl, ih (g s x)⟩

/-- The `use::state` tick loop, pure: state crosses ticks, outputs are
per-tick emissions (a Mealy-machine step, if you like automata). -/
def scanAcrossTicksTrace {ι σ β : Type _} (g : σ → ι → σ × β) : σ → List ι → List β
  | _, [] => []
  | s, x :: xs => (g s x).2 :: scanAcrossTicksTrace g (g s x).1 xs

theorem scanAcrossTicksTrace_prefix {ι σ β : Type _} (g : σ → ι → σ × β) :
    ∀ {xs ys : List ι} (s : σ), xs <+: ys →
      scanAcrossTicksTrace g s xs <+: scanAcrossTicksTrace g s ys := by
  intro xs
  induction xs with
  | nil => intro ys s _; exact List.nil_prefix
  | cons x rest ih =>
    intro ys s h
    obtain ⟨e, rfl⟩ := h
    exact List.cons_prefix_cons.mpr ⟨rfl, ih _ ⟨e, rfl⟩⟩

@[simp] theorem scanAcrossTicksTrace_length {ι σ β : Type _} (g : σ → ι → σ × β)
    (s : σ) (xs : List ι) : (scanAcrossTicksTrace g s xs).length = xs.length := by
  induction xs generalizing s with
  | nil => rfl
  | cons x rest ih => exact congrArg (· + 1) (ih _)

theorem scanAcrossTicksTrace_getElem {ι σ β : Type _} (g : σ → ι → σ × β) (s : σ)
    (xs : List ι) (t : Nat) (ht : t < (scanAcrossTicksTrace g s xs).length)
    (hx : t < xs.length) :
    (scanAcrossTicksTrace g s xs)[t] = (g ((xs.take t).foldl (fun a x => (g a x).1) s)
      (xs[t]'hx)).2 := by
  induction xs generalizing s t with
  | nil => cases hx
  | cons x rest ih =>
    cases t with
    | zero => rfl
    | succ n =>
      have hn : n < (scanAcrossTicksTrace g (g s x).1 rest).length := by
        have := ht
        simp only [scanAcrossTicksTrace, List.length_cons] at this
        omega
      have hxn : n < rest.length := by
        have := hx
        rw [List.length_cons] at this
        omega
      show (scanAcrossTicksTrace g (g s x).1 rest)[n]'hn = _
      rw [ih (g s x).1 n hn hxn]
      rfl

/-- The `use::state` states are the state scan (for relating a wire's emissions
to its state trajectory). -/
theorem scanAcrossTicksTrace_states {ι σ β : Type _} (g : σ → ι → σ × β) (s : σ)
    (xs : List ι) :
    foldAcrossTicksTrace (fun a x => (g a x).1) s xs
      = scanAcrossTicksTrace (fun a x => ((g a x).1, (g a x).1)) s xs := by
  induction xs generalizing s with
  | nil => rfl
  | cons x rest ih => exact congrArg _ (ih _)

/-! ## Trajectory-ascending traces (the realized `Monotonic` bound) -/

/-- Ascent along the realized trajectory in a value order. -/
def Ascending {σ : Type _} (vo : ValueOrder σ) (l : Trace σ) : Prop :=
  ∀ {t t' : Nat} (h : t ≤ t') (ht' : t' < l.length),
    vo.le (l[t]'(Nat.lt_of_le_of_lt h ht')) (l[t']'ht')

/-- A trace whose type carries its trajectory ascent — the realized form
of Rust's `Monotonic` singleton bound. -/
structure MonoTrace {σ : Type _} (vo : ValueOrder σ) where
  vals : Trace σ
  ascending : Ascending vo vals

/-- Equal trajectories are equal `MonoTrace`s (ascent is a proposition). -/
theorem MonoTrace.vals_ext {σ : Type _} {vo : ValueOrder σ}
    {a b : MonoTrace vo} (h : a.vals = b.vals) : a = b := by
  cases a; cases b
  cases h
  rfl

/-- Order-preserving image (Rust `SingletonMapFuncAlgebra`'s
`order_preserving`). -/
def MonoTrace.map {σ τ : Type _} {vo : ValueOrder σ}
    {vo' : ValueOrder τ} (m : MonoTrace vo) (h : σ → τ)
    (hpres : ∀ {a b}, vo.le a b → vo'.le (h a) (h b)) : MonoTrace vo' where
  vals := m.vals.map h
  ascending := by
    intro t t' hle ht'
    have hl : t' < m.vals.length := by
      have := ht'
      rwa [List.length_map] at this
    have ht : t < m.vals.length := Nat.lt_of_le_of_lt hle hl
    rw [List.getElem_map, List.getElem_map]
    exact hpres (m.ascending hle hl)

/-- The state scan of an inflationary fold, at its `Monotonic` type: the
`monotone =` obligation is paid here, once. -/
def foldAcrossTicksMonotoneTrace {ι σ : Type _} (vo : ValueOrder σ) (g : σ → ι → σ)
    (init : σ) (hinfl : ∀ s x, vo.le s (g s x)) (xs : List ι) :
    MonoTrace vo where
  vals := foldAcrossTicksTrace g init xs
  ascending := by
    intro t t' h ht'
    have ht : t < (foldAcrossTicksTrace g init xs).length := Nat.lt_of_le_of_lt h ht'
    rw [foldAcrossTicksTrace_getElem g init xs t ht,
      foldAcrossTicksTrace_getElem g init xs t' ht']
    exact vo.foldl_take_le hinfl (Nat.succ_le_succ h) init

@[simp] theorem scanMonotone_vals {ι σ : Type _} (vo : ValueOrder σ)
    (g : σ → ι → σ) (init : σ) (hinfl : ∀ s x, vo.le s (g s x))
    (xs : List ι) :
    (foldAcrossTicksMonotoneTrace vo g init hinfl xs).vals = foldAcrossTicksTrace g init xs := rfl

/-! ## Cut legality (decisions-as-inputs at consumption points)

A `batch`/`snapshot` decision is the consumed increment itself. At
`NoOrder + ExactlyOnce` the increment is a **multiset** (order already
unobservable by type) and legality is the sub-multiset lattice: consumed
so far plus the increment stays within the pool. An illegal increment
blocks — legality is realizability. -/

/-- Rust `.batch(&tick, nondet!(…))` on an unordered exactly-once
stream: realized per-tick batches. -/
def batchCuts {α : Type _} [DecidableEq α] (pool : Multiset α)
    (consumed : Multiset α) : (d : List (Multiset α)) →
      Trace (Multiset α)
  | [] => []
  | b :: ds =>
    if consumed + b ≤ pool then b :: batchCuts pool (consumed + b) ds
    else []

/-- Rust `.snapshot(&tick, nondet!(…))` of a fold over an unordered
exactly-once stream: realized accumulated views. -/
def snapshotCuts {α : Type _} [DecidableEq α] (pool : Multiset α)
    (acc : Multiset α) : (d : List (Multiset α)) →
      Trace (Multiset α)
  | [] => []
  | b :: ds =>
    if acc + b ≤ pool then (acc + b) :: snapshotCuts pool (acc + b) ds
    else []

/-- Legality only relaxes as the pool grows; realized batches are copied
verbatim — batch runs extend by prefix under pool growth. -/
theorem batchCuts_le {α : Type _} [DecidableEq α] {pool pool' : Multiset α}
    (h : pool ≤ pool') (consumed : Multiset α) (d : List (Multiset α)) :
    batchCuts pool consumed d <+: batchCuts pool' consumed d := by
  induction d generalizing consumed with
  | nil => exact List.prefix_refl _
  | cons b ds ih =>
    unfold batchCuts
    by_cases hb : consumed + b ≤ pool
    · rw [if_pos hb, if_pos (le_trans hb h)]
      exact List.cons_prefix_cons.mpr ⟨rfl, ih _⟩
    · rw [if_neg hb]
      exact List.nil_prefix

/-- Snapshot runs extend by prefix under pool growth. -/
theorem snapshotCuts_le {α : Type _} [DecidableEq α] {pool pool' : Multiset α}
    (h : pool ≤ pool') (acc : Multiset α) (d : List (Multiset α)) :
    snapshotCuts pool acc d <+: snapshotCuts pool' acc d := by
  induction d generalizing acc with
  | nil => exact List.prefix_refl _
  | cons b ds ih =>
    unfold snapshotCuts
    by_cases hb : acc + b ≤ pool
    · rw [if_pos hb, if_pos (le_trans hb h)]
      exact List.cons_prefix_cons.mpr ⟨rfl, ih _⟩
    · rw [if_neg hb]
      exact List.nil_prefix

/-- Every realized snapshot view extends the starting accumulation. -/
theorem snapshotCuts_acc_le {α : Type _} [DecidableEq α] {pool : Multiset α}
    {d : List (Multiset α)} :
    ∀ {acc v : Multiset α}, v ∈ snapshotCuts pool acc d → acc ≤ v := by
  induction d with
  | nil => intro acc v h; cases h
  | cons b ds ih =>
    intro acc v h
    unfold snapshotCuts at h
    by_cases hb : acc + b ≤ pool
    · rw [if_pos hb] at h
      rcases List.mem_cons.mp h with rfl | h'
      · exact Multiset.le_add_right acc b
      · exact le_trans (Multiset.le_add_right acc b) (ih h')
    · rw [if_neg hb] at h
      cases h

/-- Snapshot views chain along ticks (cuts accumulate). -/
theorem snapshotCuts_getElem_le {α : Type _} [DecidableEq α] {pool : Multiset α}
    {d : List (Multiset α)} :
    ∀ {acc : Multiset α} {t t' : Nat} (h : t ≤ t')
      (ht' : t' < (snapshotCuts pool acc d).length),
      (snapshotCuts pool acc d)[t]'(Nat.lt_of_le_of_lt h ht')
        ≤ (snapshotCuts pool acc d)[t']'ht' := by
  induction d with
  | nil =>
    intro acc t t' h ht'
    simp [snapshotCuts] at ht'
  | cons b ds ih =>
    intro acc t t' h ht'
    by_cases hb : acc + b ≤ pool
    · have hview : snapshotCuts pool acc (b :: ds)
          = (acc + b) :: snapshotCuts pool (acc + b) ds := by
        show (if acc + b ≤ pool then
            (acc + b) :: snapshotCuts pool (acc + b) ds else [])
          = (acc + b) :: snapshotCuts pool (acc + b) ds
        rw [if_pos hb]
      have hlen : t' < ((acc + b) :: snapshotCuts pool (acc + b) ds).length := by
        rw [← hview]; exact ht'
      have he := List.getElem_of_eq hview (Nat.lt_of_le_of_lt h ht')
      have he' := List.getElem_of_eq hview ht'
      rw [he, he']
      cases t with
      | zero =>
        cases t' with
        | zero => exact le_refl _
        | succ n =>
          have hn : n < (snapshotCuts pool (acc + b) ds).length := by
            rw [List.length_cons] at hlen
            omega
          rw [List.getElem_cons_zero, List.getElem_cons_succ]
          exact snapshotCuts_acc_le (List.getElem_mem hn)
      | succ m =>
        cases t' with
        | zero => omega
        | succ n =>
          have hn : n < (snapshotCuts pool (acc + b) ds).length := by
            rw [List.length_cons] at hlen
            omega
          rw [List.getElem_cons_succ, List.getElem_cons_succ]
          exact ih (Nat.le_of_succ_le_succ h) hn
    · unfold snapshotCuts at ht'
      rw [if_neg hb] at ht'
      cases ht'

theorem snapshotCuts_length_le {α : Type _} [DecidableEq α] {pool : Multiset α} :
    ∀ (d : List (Multiset α)) (acc : Multiset α),
      (snapshotCuts pool acc d).length ≤ d.length := by
  intro d
  induction d with
  | nil => intro _; exact Nat.le_refl _
  | cons b ds ih =>
    intro acc
    unfold snapshotCuts
    by_cases hb : acc + b ≤ pool
    · rw [if_pos hb, List.length_cons]
      exact Nat.succ_le_succ (ih _)
    · rw [if_neg hb]
      exact Nat.zero_le _

/-- `snapshotCuts` realizes every tick whose cumulative consumption fits
the pool. -/
theorem snapshotCuts_all_of_le {α : Type _} [DecidableEq α] {pool : Multiset α} :
    ∀ (d : List (Multiset α)) (acc : Multiset α),
      acc + d.sum ≤ pool →
      (snapshotCuts pool acc d).length = d.length := by
  intro d
  induction d with
  | nil => intro _ _; rfl
  | cons b ds ih =>
    intro acc h
    rw [List.sum_cons, ← Multiset.add_assoc] at h
    have hb : acc + b ≤ pool :=
      le_trans (Multiset.le_add_right (acc + b) ds.sum) h
    unfold snapshotCuts
    rw [if_pos hb, List.length_cons]
    exact congrArg (· + 1) (ih (acc + b) h)

/-- `batchCuts` copies an all-legal decision verbatim. -/
theorem batchCuts_all_of_le {α : Type _} [DecidableEq α] {pool : Multiset α} :
    ∀ (d : List (Multiset α)) (consumed : Multiset α),
      consumed + d.sum ≤ pool →
      batchCuts pool consumed d = d := by
  intro d
  induction d with
  | nil => intro _ _; rfl
  | cons b ds ih =>
    intro consumed h
    rw [List.sum_cons, ← Multiset.add_assoc] at h
    have hb : consumed + b ≤ pool :=
      le_trans (Multiset.le_add_right (consumed + b) ds.sum) h
    unfold batchCuts
    rw [if_pos hb]
    exact congrArg (b :: ·) (ih (consumed + b) h)

/-- Prefix cuts of an ordered pool (`snapshot` of an ordered fold): the
decision is the per-tick element count; a cut past the realized content
blocks. -/
def prefixCuts {α : Type _} (pool : List α) (acc : Nat) :
    (d : List Nat) → Trace (List α)
  | [] => []
  | n :: ds =>
    if acc + n ≤ pool.length then
      pool.take (acc + n) :: prefixCuts pool (acc + n) ds
    else []

theorem prefixCuts_le {α : Type _} {pool pool' : List α}
    (h : pool <+: pool') (acc : Nat) (d : List Nat) :
    prefixCuts pool acc d <+: prefixCuts pool' acc d := by
  induction d generalizing acc with
  | nil => exact List.prefix_refl _
  | cons n ds ih =>
    unfold prefixCuts
    by_cases hn : acc + n ≤ pool.length
    · rw [if_pos hn, if_pos (Nat.le_trans hn h.length_le)]
      refine List.cons_prefix_cons.mpr ⟨?_, ih _⟩
      obtain ⟨e, rfl⟩ := h
      rw [List.take_append_of_le_length hn]
    · rw [if_neg hn]
      exact List.nil_prefix

theorem take_prefix_take {α : Type _} {l : List α} {m n : Nat}
    (h : m ≤ n) : l.take m <+: l.take n := by
  refine ⟨(l.take n).drop m, ?_⟩
  rw [show l.take m = (l.take n).take m from by
      rw [List.take_take, Nat.min_eq_left h],
    List.take_append_drop]

theorem prefixCuts_acc_prefix {α : Type _} {pool : List α}
    {d : List Nat} :
    ∀ {acc : Nat} {v : List α}, v ∈ prefixCuts pool acc d →
      pool.take acc <+: v := by
  induction d with
  | nil => intro acc v h; cases h
  | cons n ds ih =>
    intro acc v h
    unfold prefixCuts at h
    by_cases hn : acc + n ≤ pool.length
    · rw [if_pos hn] at h
      rcases List.mem_cons.mp h with rfl | h'
      · exact take_prefix_take (Nat.le_add_right acc n)
      · exact (take_prefix_take (Nat.le_add_right acc n)).trans (ih h')
    · rw [if_neg hn] at h
      cases h

theorem prefixCuts_getElem_prefix {α : Type _} {pool : List α}
    {d : List Nat} :
    ∀ {acc : Nat} {t t' : Nat} (h : t ≤ t')
      (ht' : t' < (prefixCuts pool acc d).length),
      (prefixCuts pool acc d)[t]'(Nat.lt_of_le_of_lt h ht')
        <+: (prefixCuts pool acc d)[t']'ht' := by
  induction d with
  | nil =>
    intro acc t t' h ht'
    simp [prefixCuts] at ht'
  | cons n ds ih =>
    intro acc t t' h ht'
    by_cases hn : acc + n ≤ pool.length
    · have hview : prefixCuts pool acc (n :: ds)
          = pool.take (acc + n) :: prefixCuts pool (acc + n) ds := by
        show (if acc + n ≤ pool.length then
            pool.take (acc + n) :: prefixCuts pool (acc + n) ds else [])
          = pool.take (acc + n) :: prefixCuts pool (acc + n) ds
        rw [if_pos hn]
      have hlen : t' < (pool.take (acc + n)
          :: prefixCuts pool (acc + n) ds).length := by
        rw [← hview]; exact ht'
      have he := List.getElem_of_eq hview (Nat.lt_of_le_of_lt h ht')
      have he' := List.getElem_of_eq hview ht'
      rw [he, he']
      cases t with
      | zero =>
        cases t' with
        | zero => exact List.prefix_refl _
        | succ m =>
          have hm : m < (prefixCuts pool (acc + n) ds).length := by
            rw [List.length_cons] at hlen
            omega
          rw [List.getElem_cons_zero, List.getElem_cons_succ]
          exact prefixCuts_acc_prefix (List.getElem_mem hm)
      | succ u =>
        cases t' with
        | zero => omega
        | succ m =>
          have hm : m < (prefixCuts pool (acc + n) ds).length := by
            rw [List.length_cons] at hlen
            omega
          rw [List.getElem_cons_succ, List.getElem_cons_succ]
          exact ih (Nat.le_of_succ_le_succ h) hm
    · unfold prefixCuts at ht'
      rw [if_neg hn] at ht'
      cases ht'

/-- Membership-legal snapshot reads (`AtLeastOnce`): an increment may
duplicate freely but only quote the pool. -/
def snapshotMemCuts {α : Type _} [DecidableEq α] (pool : Multiset α)
    (acc : Multiset α) : (d : List (Multiset α)) → Trace (Multiset α)
  | [] => []
  | b :: ds =>
    if ∀ x ∈ b, x ∈ pool then
      (acc + b) :: snapshotMemCuts pool (acc + b) ds
    else []

theorem snapshotMemCuts_acc_le {α : Type _} [DecidableEq α]
    {pool : Multiset α} {d : List (Multiset α)} :
    ∀ {acc v : Multiset α}, v ∈ snapshotMemCuts pool acc d → acc ≤ v := by
  induction d with
  | nil => intro acc v h; cases h
  | cons b ds ih =>
    intro acc v h
    unfold snapshotMemCuts at h
    by_cases hb : ∀ x ∈ b, x ∈ pool
    · rw [if_pos hb] at h
      rcases List.mem_cons.mp h with rfl | h'
      · exact Multiset.le_add_right acc b
      · exact le_trans (Multiset.le_add_right acc b) (ih h')
    · rw [if_neg hb] at h
      cases h

theorem snapshotMemCuts_getElem_le {α : Type _} [DecidableEq α]
    {pool : Multiset α} {d : List (Multiset α)} :
    ∀ {acc : Multiset α} {t t' : Nat} (h : t ≤ t')
      (ht' : t' < (snapshotMemCuts pool acc d).length),
      (snapshotMemCuts pool acc d)[t]'(Nat.lt_of_le_of_lt h ht')
        ≤ (snapshotMemCuts pool acc d)[t']'ht' := by
  induction d with
  | nil =>
    intro acc t t' h ht'
    simp [snapshotMemCuts] at ht'
  | cons b ds ih =>
    intro acc t t' h ht'
    by_cases hb : ∀ x ∈ b, x ∈ pool
    · have hview : snapshotMemCuts pool acc (b :: ds)
          = (acc + b) :: snapshotMemCuts pool (acc + b) ds := by
        show (if ∀ x ∈ b, x ∈ pool then
            (acc + b) :: snapshotMemCuts pool (acc + b) ds else [])
          = (acc + b) :: snapshotMemCuts pool (acc + b) ds
        rw [if_pos hb]
      have hlen : t' < ((acc + b)
          :: snapshotMemCuts pool (acc + b) ds).length := by
        rw [← hview]; exact ht'
      have he := List.getElem_of_eq hview (Nat.lt_of_le_of_lt h ht')
      have he' := List.getElem_of_eq hview ht'
      rw [he, he']
      cases t with
      | zero =>
        cases t' with
        | zero => exact le_refl _
        | succ m =>
          have hm : m < (snapshotMemCuts pool (acc + b) ds).length := by
            rw [List.length_cons] at hlen
            omega
          rw [List.getElem_cons_zero, List.getElem_cons_succ]
          exact snapshotMemCuts_acc_le (List.getElem_mem hm)
      | succ u =>
        cases t' with
        | zero => omega
        | succ m =>
          have hm : m < (snapshotMemCuts pool (acc + b) ds).length := by
            rw [List.length_cons] at hlen
            omega
          rw [List.getElem_cons_succ, List.getElem_cons_succ]
          exact ih (Nat.le_of_succ_le_succ h) hm
    · unfold snapshotMemCuts at ht'
      rw [if_neg hb] at ht'
      cases ht'

/-- `assume_ordering`'s selection: realize unordered exactly-once
content as a sequence by drawing without replacement; an illegal pick
blocks. -/
def selectOrder {α : Type _} [DecidableEq α] (pool : Multiset α) :
    List α → List α
  | [] => []
  | x :: xs => if x ∈ pool then x :: selectOrder (pool.erase x) xs else []

/-- Every view an ordered read exposes is a prefix-take of the pool. -/
theorem prefixCuts_mem_take {α : Type _} {pool : List α} {d : List Nat} :
    ∀ {acc : Nat} {v : List α}, v ∈ prefixCuts pool acc d →
      ∃ k, v = pool.take k := by
  induction d with
  | nil => intro acc v h; cases h
  | cons n ds ih =>
    intro acc v h
    unfold prefixCuts at h
    by_cases hle : acc + n ≤ pool.length
    · rw [if_pos hle] at h
      rcases List.mem_cons.mp h with rfl | h'
      · exact ⟨acc + n, rfl⟩
      · exact ih h'
    · rw [if_neg hle] at h
      cases h

/-- Rust `.batch(&tick, nondet!(…))` of an **ordered** stream: per-tick
consumed slice sizes; a cut is legal while it stays within the realized
pool (blocking beyond). -/
def sliceCuts {α : Type _} (pool : List α) (consumed : Nat) :
    (d : List Nat) → Trace (List α)
  | [] => []
  | n :: ds =>
    if consumed + n ≤ pool.length then
      (pool.drop consumed).take n :: sliceCuts pool (consumed + n) ds
    else []

/-- Realized slices are copied verbatim as the pool grows. -/
theorem sliceCuts_le {α : Type _} {pool pool' : List α}
    (h : pool <+: pool') (consumed : Nat) (d : List Nat) :
    sliceCuts pool consumed d <+: sliceCuts pool' consumed d := by
  induction d generalizing consumed with
  | nil => exact List.prefix_refl _
  | cons n ds ih =>
    unfold sliceCuts
    by_cases hle : consumed + n ≤ pool.length
    · rw [if_pos hle, if_pos (Nat.le_trans hle h.length_le)]
      obtain ⟨e, rfl⟩ := h
      refine List.cons_prefix_cons.mpr ⟨?_, ih _⟩
      have hdrop : (pool ++ e).drop consumed
          = pool.drop consumed ++ e := List.drop_append_of_le_length
        (by omega)
      rw [hdrop, List.take_append_of_le_length (by
        rw [List.length_drop]
        omega)]
    · rw [if_neg hle]
      exact List.nil_prefix

/-- Selections extend under pool growth. -/
theorem selectOrder_le {α : Type _} [DecidableEq α]
    {pool pool' : Multiset α} (h : pool ≤ pool') (d : List α) :
    selectOrder pool d <+: selectOrder pool' d := by
  induction d generalizing pool pool' with
  | nil => exact List.prefix_refl _
  | cons x xs ih =>
    unfold selectOrder
    by_cases hx : x ∈ pool
    · rw [if_pos hx, if_pos (Multiset.mem_of_le h hx)]
      exact List.cons_prefix_cons.mpr
        ⟨rfl, ih (Multiset.erase_le_erase x h)⟩
    · rw [if_neg hx]
      exact List.nil_prefix

/-- Inflationary steps fold a multiset upward (any representative order
— the fold exists via commutativity). -/
theorem multiset_le_foldl {α σ : Type _} (vo : ValueOrder σ)
    (g : σ → α → σ) (comm : ∀ s x y, g (g s x) y = g (g s y) x)
    (hinfl : ∀ s x, vo.le s (g s x)) (e : Multiset α) (s : σ) :
    vo.le s (@Multiset.foldl α σ g ⟨fun a x y => comm a x y⟩ s e) := by
  induction e using Multiset.induction_on generalizing s with
  | empty => exact vo.le_refl s
  | cons x m ih =>
    rw [Multiset.foldl_cons]
    exact vo.le_trans (hinfl s x) (ih (g s x))

/-- The sub-multiset growth order, packaged. -/
def ValueOrder.multiset (α : Type _) : ValueOrder (Multiset α) where
  le a b := a ≤ b
  le_refl _ := _root_.le_refl _
  le_trans h₁ h₂ := _root_.le_trans h₁ h₂

/-- `(s + {x}) + {y} = (s + {y}) + {x}` (the singleton-append
commutativity the entry pool pays). -/
theorem add_singleton_comm {α : Type _} (s : Multiset α) (x y : α) :
    s + {x} + {y} = s + {y} + {x} := by
  rw [Multiset.add_assoc, Multiset.add_assoc,
    Multiset.add_comm ({x} : Multiset α) {y}]

/-- Accumulating a batch element-wise is accumulating the batch. -/
theorem foldl_add_singleton {α : Type _} (s b : Multiset α) :
    @Multiset.foldl _ _ (fun s e => s + {e})
      ⟨fun s x y => add_singleton_comm s x y⟩ s b = s + b := by
  induction b using Multiset.induction_on generalizing s with
  | empty => rw [Multiset.foldl_zero, Multiset.add_zero]
  | cons x m ih =>
    rw [Multiset.foldl_cons, ih, ← Multiset.singleton_add,
      ← Multiset.add_assoc]

/-- Tracing a multiset-sum's members back to their summands. -/
theorem mem_list_sum {α : Type _} {x : α} :
    ∀ {l : List (Multiset α)}, x ∈ l.sum ↔ ∃ m ∈ l, x ∈ m
  | [] => by simp
  | m :: rest => by
    rw [List.sum_cons, Multiset.mem_add]
    constructor
    · rintro (h | h)
      · exact ⟨m, List.mem_cons_self .., h⟩
      · obtain ⟨m', hm', hx⟩ := mem_list_sum.mp h
        exact ⟨m', List.mem_cons_of_mem _ hm', hx⟩
    · rintro ⟨m', hm', hx⟩
      rcases List.mem_cons.mp hm' with rfl | hm'
      · exact Or.inl hx
      · exact Or.inr (mem_list_sum.mpr ⟨m', hm', hx⟩)

/-- Sums of multiset traces grow along trace prefixes. -/
theorem sum_le_sum_of_prefix {α : Type _} :
    ∀ {v w : List (Multiset α)}, v <+: w → v.sum ≤ w.sum
  | [], w, _ => by
    rw [List.sum_nil]
    exact Multiset.zero_le _
  | x :: v', x' :: w', h => by
    obtain ⟨rfl, h'⟩ := List.cons_prefix_cons.mp h
    rw [List.sum_cons, List.sum_cons]
    exact Multiset.add_le_add_left (sum_le_sum_of_prefix h')
  | x :: v', [], h => by cases List.prefix_nil.mp h

/-! ## Guarded cycles (Rust `forward_ref`) -/

/-- Kleene iteration from the least wire. -/
def iterate {τ : Type _} (F : τ → τ) (x : τ) : Nat → τ
  | 0 => x
  | k + 1 => F (iterate F x k)

@[simp] theorem iterate_zero {τ : Type _} (F : τ → τ) (x : τ) :
    iterate F x 0 = x := rfl

@[simp] theorem iterate_succ {τ : Type _} (F : τ → τ) (x : τ) (k : Nat) :
    iterate F x (k + 1) = F (iterate F x k) := rfl

/-- One Kleene step ascends: if the seed is below its image and the body
preserves the order, each iterate is below the next. -/
theorem iterate_le_succ {τ : Type _} {R : τ → τ → Prop} {F : τ → τ}
    {x : τ} (hx : R x (F x)) (hF : ∀ {a b}, R a b → R (F a) (F b)) :
    ∀ k, R (iterate F x k) (iterate F x (k + 1))
  | 0 => hx
  | k + 1 => hF (iterate_le_succ hx hF k)

/-- **The Kleene chain** (variable-fuel monotonicity through a `fix`):
for a reflexive-transitive `R` preserved by the body from an ascending
seed, deeper unfoldings extend shallower ones. The preservation premise
is discharged by instantiating the SAME body text at `MonoRel` (the
iterate-projection principle). -/
theorem iterate_chain {τ : Type _} {R : τ → τ → Prop}
    (hrefl : ∀ a, R a a)
    (htrans : ∀ {a b c}, R a b → R b c → R a c)
    {F : τ → τ} {x : τ} (hx : R x (F x))
    (hF : ∀ {a b}, R a b → R (F a) (F b))
    {k k' : Nat} (h : k ≤ k') :
    R (iterate F x k) (iterate F x k') := by
  induction k' with
  | zero => cases Nat.le_zero.mp h; exact hrefl _
  | succ k' ih =>
    rcases Nat.lt_or_ge k (k' + 1) with hlt | hge
    · exact htrans (ih (Nat.le_of_lt_succ hlt))
        (iterate_le_succ hx (fun {a b} => hF) k')
    · have : k = k' + 1 := Nat.le_antisymm h hge
      subst this
      exact hrefl _

/-- **Fixpoint induction** (the ONE generic cycle induction): a
property of the seed preserved by the body holds at every unfolding
depth. Safety facts about `fix`-closed wires are its instances. -/
theorem fix_induction {τ : Type _} {P : τ → Prop} {F : τ → τ} {x : τ}
    (hx : P x) (hF : ∀ w, P w → P (F w)) :
    ∀ k, P (iterate F x k)
  | 0 => hx
  | k + 1 => hF _ (fix_induction hx hF k)

/-- **Iterate projection** (the iterate-projection principle): an
iteration on a carrier that projects to a pair — e.g. `MonoRel`'s
coupled subtype via `.val` — projects to the two component iterations,
provided the step commutes with the projection (for bodies built from
`MonoRel` ops, that commutation is `rfl`). NOT definitional at variable
fuel; this is the bridge that lets `fix`-closed programs inherit the
`MonoRel` two-liner. -/
theorem iterate_val_proj {γ α β : Type _} (val : γ → α × β)
    (F : γ → γ) (F1 : α → α) (F2 : β → β)
    (hF : ∀ z, val (F z) = (F1 (val z).1, F2 (val z).2))
    (z0 : γ) :
    ∀ k, val (iterate F z0 k)
      = (iterate F1 (val z0).1 k, iterate F2 (val z0).2 k)
  | 0 => rfl
  | k + 1 => by
    show val (F (iterate F z0 k)) = _
    rw [hF (iterate F z0 k), iterate_val_proj val F F1 F2 hF z0 k]
    rfl

/-- Once the Kleene chain repeats, it is constant: stability
propagates. -/
theorem iterate_stab_of_fixed {τ : Type _} {F : τ → τ} {x : τ} {N : Nat}
    (hfix : iterate F x (N + 1) = iterate F x N) :
    ∀ {k : Nat}, N ≤ k → iterate F x k = iterate F x N := by
  intro k hk
  induction k with
  | zero => cases Nat.le_zero.mp hk; rfl
  | succ k ih =>
    rcases Nat.lt_or_ge N (k + 1) with hlt | hge
    · have hNk : N ≤ k := Nat.le_of_lt_succ hlt
      show F (iterate F x k) = _
      rw [ih hNk]
      exact hfix
    · have : N = k + 1 := Nat.le_antisymm hk hge
      subst this
      rfl

/-- **Stabilization** (`fix_stabilizes`): if every non-fixed Kleene step
strictly grows a measure that is bounded on the chain, the chain reaches
a TRUE fixpoint within the budget — the wire's value stops being
fuel-dependent. (The staged 6b semantics: `fix` as stabilization search;
recorded in `HydroV2/README.md`.) -/
theorem fix_stabilizes {τ : Type _} {F : τ → τ} {x : τ}
    (μ : τ → Nat) (B : Nat)
    (hgrow : ∀ k, F (iterate F x k) = iterate F x k ∨
      μ (iterate F x k) < μ (iterate F x (k + 1)))
    (hbound : ∀ k, μ (iterate F x k) ≤ B) :
    ∃ N, N ≤ B + 1 ∧ iterate F x (N + 1) = iterate F x N := by
  by_contra h
  push Not at h
  -- every step up to B + 1 strictly grows the measure, so μ climbs past B
  have hclimb : ∀ k, k ≤ B + 1 → k + μ x ≤ μ (iterate F x k) := by
    intro k hk
    induction k with
    | zero => simp
    | succ k ih =>
      have hne := h k (Nat.le_of_succ_le hk)
      rcases hgrow k with hfix | hlt
      · exact absurd hfix hne
      · have := ih (Nat.le_of_succ_le hk)
        omega
  have := hclimb (B + 1) (Nat.le_refl _)
  have := hbound (B + 1)
  omega


/-- Every selected element was in the pool. -/
theorem selectOrder_mem {α : Type _} [DecidableEq α] :
    ∀ {pool : Multiset α} {d : List α}, ∀ x ∈ selectOrder pool d, x ∈ pool
  | pool, [], x, h => absurd h (List.not_mem_nil)
  | pool, y :: ys, x, h => by
    unfold selectOrder at h
    by_cases hy : y ∈ pool
    · rw [if_pos hy] at h
      rcases List.mem_cons.mp h with rfl | h'
      · exact hy
      · exact Multiset.mem_of_mem_erase (selectOrder_mem x h')
    · rw [if_neg hy] at h
      cases h

/-- The selection is a sub-multiset of the pool (multiplicity form of
selection legality). -/
theorem selectOrder_subpool {α : Type _} [DecidableEq α] :
    ∀ {pool : Multiset α} {d : List α},
      (Multiset.ofList (selectOrder pool d)) ≤ pool
  | pool, [] => Multiset.zero_le pool
  | pool, y :: ys => by
    unfold selectOrder
    by_cases hy : y ∈ pool
    · rw [if_pos hy]
      show Multiset.ofList (y :: selectOrder (pool.erase y) ys) ≤ pool
      rw [show Multiset.ofList (y :: selectOrder (pool.erase y) ys)
          = y ::ₘ Multiset.ofList (selectOrder (pool.erase y) ys) from rfl]
      exact le_trans
        (Multiset.cons_le_cons y
          (selectOrder_subpool (pool := pool.erase y) (d := ys)))
        (le_of_eq (Multiset.cons_erase hy))
    · rw [if_neg hy]
      exact Multiset.zero_le pool

/-- The consumed multiset of a batch run stays within the pool (the
count-legality invariant, accumulated). -/
theorem batchCuts_sum_le {α : Type _} [DecidableEq α] {pool : Multiset α} :
    ∀ {d : List (Multiset α)} {consumed : Multiset α},
      consumed ≤ pool →
      consumed + (batchCuts pool consumed d).sum ≤ pool
  | [], consumed, h => by
    show consumed + ([] : Trace (Multiset α)).sum ≤ pool
    simpa using h
  | b :: ds, consumed, h => by
    unfold batchCuts
    by_cases hb : consumed + b ≤ pool
    · rw [if_pos hb]
      have := batchCuts_sum_le (d := ds) hb
      rw [List.sum_cons, ← Multiset.add_assoc]
      exact this
    · rw [if_neg hb]
      simpa using h

/-- Pigeonhole for capped contributions: if each index contributes at
most one and the total reaches `K`, at least `K` distinct indices
contribute (the distinct-quorum extraction). -/
theorem filter_pos_length_of_sum_ge {ι : Type _} :
    ∀ (l : List ι) (c : ι → Nat), (∀ j ∈ l, c j ≤ 1) →
      ∀ {K : Nat}, K ≤ (l.map c).sum →
      K ≤ (l.filter (fun j => decide (c j ≠ 0))).length
  | [], _, _, K, h => by simpa using h
  | a :: l, c, hcap, K, h => by
    rw [List.map_cons, List.sum_cons] at h
    have hihcap : ∀ j ∈ l, c j ≤ 1 :=
      fun j hj => hcap j (List.mem_cons_of_mem a hj)
    by_cases ha : c a = 0
    · rw [List.filter_cons_of_neg (by simpa using ha)]
      exact filter_pos_length_of_sum_ge l c hihcap
        (by rw [ha, Nat.zero_add] at h; exact h)
    · rw [List.filter_cons_of_pos (by simpa using ha), List.length_cons]
      have hK1 : K - 1 ≤ (l.map c).sum := by
        have := hcap a (List.mem_cons_self)
        omega
      have := filter_pos_length_of_sum_ge l c hihcap hK1
      omega

/-- **Distinct representatives** from unit-capped summands: a
sub-multiset of a sum whose summands each hold at most one copy is
covered by distinct indices, one witness copy each. -/
theorem exists_distinct_reps {ι α : Type _} [DecidableEq α] :
    ∀ (l : List ι) (q : ι → Multiset α) (m : Multiset α),
      l.Nodup → m ≤ (l.map q).sum → (∀ j ∈ l, (q j).card ≤ 1) →
      ∃ S : List ι, S.Nodup ∧ S ⊆ l ∧ Multiset.card m ≤ S.length ∧
        ∀ j ∈ S, ∃ x ∈ m, x ∈ q j
  | [], _, m, _, hle, _ => by
    refine ⟨[], List.nodup_nil, fun x hx => hx, ?_, fun j hj => nomatch hj⟩
    rw [List.map_nil, List.sum_nil] at hle
    rw [Multiset.le_zero.mp hle]
    exact Nat.le_refl _
  | a :: rest, q, m, hnd, hle, hcap => by
    rw [List.map_cons, List.sum_cons] at hle
    have hnd' := List.nodup_cons.mp hnd
    have hsub : m - q a ≤ (rest.map q).sum := by
      rw [Multiset.sub_le_iff_le_add']
      exact hle
    obtain ⟨S', hS'nd, hS'sub, hS'len, hS'rep⟩ :=
      exists_distinct_reps rest q (m - q a) hnd'.2 hsub
        (fun j hj => hcap j (List.mem_cons_of_mem a hj))
    by_cases hinter : m ∩ q a = 0
    · -- disjoint from `a`'s summand: `m` survives the subtraction whole
      have hm : m - q a = m := by
        have h0 := Multiset.sub_add_inter m (q a)
        rw [hinter, Multiset.add_zero] at h0
        exact h0
      refine ⟨S', hS'nd, fun x hx => List.mem_cons_of_mem a (hS'sub hx),
        ?_, fun j hj => ?_⟩
      · rw [← hm]; exact hS'len
      · obtain ⟨x, hx, hxq⟩ := hS'rep j hj
        exact ⟨x, Multiset.mem_of_le (Multiset.sub_le_iff_le_add.mpr (Multiset.le_add_right _ _)) hx, hxq⟩
    · -- `a` contributes: prepend it, with a witness from the overlap
      obtain ⟨x, hx⟩ := Multiset.exists_mem_of_ne_zero hinter
      have hxm : x ∈ m := (Multiset.mem_inter.mp hx).1
      have hxa : x ∈ q a := (Multiset.mem_inter.mp hx).2
      refine ⟨a :: S', List.nodup_cons.mpr
        ⟨fun hc => hnd'.1 (hS'sub hc), hS'nd⟩,
        fun y hy => (List.mem_cons.mp hy).elim (fun h => h ▸
          List.mem_cons_self) (fun h => List.mem_cons_of_mem a (hS'sub h)),
        ?_, fun j hj => ?_⟩
      · have hcard : Multiset.card m
            = Multiset.card (m - q a) + Multiset.card (m ∩ q a) := by
          rw [← Multiset.card_add, Multiset.sub_add_inter]
        have hia : Multiset.card (m ∩ q a) ≤ 1 :=
          le_trans (Multiset.card_le_card Multiset.inter_le_right)
            (hcap a List.mem_cons_self)
        rw [List.length_cons]
        omega
      · rcases List.mem_cons.mp hj with rfl | hj'
        · exact ⟨x, hxm, hxa⟩
        · obtain ⟨y, hy, hyq⟩ := hS'rep j hj'
          exact ⟨y, Multiset.mem_of_le (Multiset.sub_le_iff_le_add.mpr (Multiset.le_add_right _ _)) hy, hyq⟩

/-- Parameterized Kleene monotonicity: coupled iterations from related
seeds through a preserving body pair stay related at every equal
fuel. -/
theorem iterate_mono_param {τ : Type _} {R : τ → τ → Prop}
    {F F' : τ → τ} (hFF' : ∀ {a b}, R a b → R (F a) (F' b))
    {x x' : τ} (hx : R x x') :
    ∀ k, R (iterate F x k) (iterate F' x' k)
  | 0 => hx
  | k + 1 => hFF' (iterate_mono_param (fun {a b} => hFF') hx k)

/-! ## Multiset counting toolkit (contract-side) -/


theorem countP_impl_le {α : Type _} [DecidableEq α]
    (s : Multiset α) (p q : α → Prop) [DecidablePred p]
    [DecidablePred q] (h : ∀ a, p a → q a) :
    s.countP p ≤ s.countP q := by
  induction s using Multiset.induction_on with
  | empty => exact le_refl _
  | cons a m ih =>
    rw [Multiset.countP_cons, Multiset.countP_cons]
    by_cases hp : p a
    · rw [if_pos hp, if_pos (h a hp)]
      omega
    · rw [if_neg hp]
      by_cases hq : q a
      · rw [if_pos hq]
        omega
      · rw [if_neg hq]
        omega


theorem sublist_sum_le {α : Type _} [DecidableEq α]
    {l₁ l₂ : List (Multiset α)} (h : l₁.Sublist l₂) :
    l₁.sum ≤ l₂.sum := by
  induction h with
  | slnil => exact le_refl _
  | cons a _ ih =>
    rw [List.sum_cons]
    exact le_trans ih (Multiset.le_add_left _ _)
  | cons_cons a _ ih =>
    rw [List.sum_cons, List.sum_cons]
    exact Multiset.add_le_add_left ih


theorem map_fst_zip_prefix {α β : Type _} :
    ∀ (a : List α) (b : List β), (List.zip a b).map Prod.fst <+: a
  | [], _ => List.nil_prefix
  | _ :: _, [] => List.nil_prefix
  | x :: xs, y :: ys => by
    rw [List.zip_cons_cons, List.map_cons]
    exact List.cons_prefix_cons.mpr ⟨rfl, map_fst_zip_prefix xs ys⟩


theorem nodup_keys_inj {α β : Type _} {l : List (α × β)}
    (h : (l.map Prod.fst).Nodup) {x y : α × β}
    (hx : x ∈ l) (hy : y ∈ l) (hxy : x.1 = y.1) : x = y := by
  obtain ⟨ix, hix, rfl⟩ := List.mem_iff_getElem.mp hx
  obtain ⟨iy, hiy, rfl⟩ := List.mem_iff_getElem.mp hy
  by_contra hne
  have hine : ix ≠ iy := fun heq => hne (by subst heq; rfl)
  have hpw := List.pairwise_iff_getElem.mp h
  have hmx : ix < (l.map Prod.fst).length := by simpa using hix
  have hmy : iy < (l.map Prod.fst).length := by simpa using hiy
  rcases Nat.lt_or_gt_of_ne hine with hlt | hgt
  · have hp := hpw ix iy hmx hmy hlt
    rw [List.getElem_map, List.getElem_map] at hp
    exact hp hxy
  · have hp := hpw iy ix hmy hmx hgt
    rw [List.getElem_map, List.getElem_map] at hp
    exact hp hxy.symm


theorem countP_key_le_one {α β : Type _} [DecidableEq α]
    [DecidableEq β] {l : List (α × β)}
    (h : (l.map Prod.fst).Nodup) (k : α) :
    (Multiset.ofList l).countP (fun x => x.1 = k) ≤ 1 := by
  induction l with
  | nil => simp
  | cons x xs ih =>
    rw [show (Multiset.ofList (x :: xs)) = x ::ₘ Multiset.ofList xs
      from rfl, Multiset.countP_cons]
    rw [List.map_cons, List.nodup_cons] at h
    by_cases hx : x.1 = k
    · rw [if_pos hx]
      have hz : (Multiset.ofList xs).countP (fun y => y.1 = k) = 0 := by
        rw [Multiset.countP_eq_zero]
        intro y hy hyk
        exact h.1 (by
          rw [hx, ← hyk]
          exact List.mem_map_of_mem (Multiset.mem_coe.mp hy))
      omega
    · rw [if_neg hx]
      have := ih h.2
      omega



private theorem list_sum_zero : ∀ {l : List Nat}, (∀ n ∈ l, n = 0) →
    l.sum = 0
  | [], _ => rfl
  | n :: ns, h => by
    rw [List.sum_cons, h n (List.mem_cons_self ..),
      list_sum_zero (fun m hm => h m (List.mem_cons_of_mem _ hm))]

/-- A list-sum of naturals bounded by a single distinguished index. -/
theorem sum_map_le_single {ι : Type _} [DecidableEq ι] :
    ∀ {l : List ι}, l.Nodup → ∀ (f : ι → Nat) (x : ι) {k : Nat},
    (∀ y ∈ l, y ≠ x → f y = 0) → f x ≤ k → (l.map f).sum ≤ k
  | [], _, _, _, _, _, _ => Nat.zero_le _
  | y :: ys, hnd, f, x, k, h0, hx => by
    rw [List.map_cons, List.sum_cons]
    by_cases hyx : y = x
    · subst hyx
      have hzero : ((ys.map f).sum) = 0 := by
        refine list_sum_zero ?_
        intro n hn
        obtain ⟨z, hz, rfl⟩ := List.mem_map.mp hn
        exact h0 z (List.mem_cons_of_mem _ hz)
          (fun hzy => ((List.nodup_cons.mp hnd).1 (hzy ▸ hz)))
      omega
    · rw [h0 y (List.mem_cons_self ..) hyx]
      have := sum_map_le_single (List.nodup_cons.mp hnd).2 f x
        (fun z hz hzx => h0 z (List.mem_cons_of_mem _ hz) hzx) hx
      omega

/-- `filterMap` distributes over a list-sum of multisets. -/
theorem filterMap_list_sum {α β : Type _} [DecidableEq α] [DecidableEq β]
    (f : α → Option β) :
    ∀ (l : List (Multiset α)),
      l.sum.filterMap f = (l.map (fun m => m.filterMap f)).sum
  | [] => by simp
  | m :: ms => by
    rw [List.sum_cons, Multiset.filterMap_add, List.map_cons,
      List.sum_cons, filterMap_list_sum f ms]

/-- `filter` distributes over a list-sum of multisets. -/
theorem filter_list_sum {α : Type _} [DecidableEq α]
    (p : α → Prop) [DecidablePred p] :
    ∀ (l : List (Multiset α)),
      l.sum.filter p = (l.map (fun m => m.filter p)).sum
  | [] => by simp
  | m :: ms => by
    rw [List.sum_cons, Multiset.filter_add, List.map_cons,
      List.sum_cons, filter_list_sum p ms]


theorem count_list_sum {α : Type _} [DecidableEq α] (b : α) :
    ∀ (l : List (Multiset α)), l.sum.count b = (l.map (·.count b)).sum
  | [] => rfl
  | m :: ms => by
    rw [List.sum_cons, Multiset.count_add, List.map_cons, List.sum_cons,
      count_list_sum b ms]


theorem countP_list_sum {α : Type _} [DecidableEq α]
    (p : α → Prop) [DecidablePred p] :
    ∀ (l : List (Multiset α)), l.sum.countP p = (l.map (·.countP p)).sum
  | [] => rfl
  | m :: ms => by
    rw [List.sum_cons, Multiset.countP_add, List.map_cons, List.sum_cons,
      countP_list_sum p ms]


theorem countP_filterMap_le {α β : Type _} [DecidableEq α]
    [DecidableEq β] (f : α → Option β) (p : β → Prop) [DecidablePred p]
    (q : α → Prop) [DecidablePred q]
    (h : ∀ a b', f a = some b' → p b' → q a) (S : Multiset α) :
    (S.filterMap f).countP p ≤ S.countP q := by
  induction S using Multiset.induction_on with
  | empty => rfl
  | cons a S ih =>
    rw [Multiset.filterMap_cons, Multiset.countP_cons]
    cases hfa : f a with
    | none =>
      rw [show ((Option.map (fun b => ({b} : Multiset β)) none).getD 0)
        = (0 : Multiset β) from rfl, Multiset.zero_add]
      exact le_trans ih (Nat.le_add_right _ _)
    | some b' =>
      rw [show ((Option.map (fun b => ({b} : Multiset β)) (some b')).getD 0)
        = ({b'} : Multiset β) from rfl, Multiset.singleton_add,
        Multiset.countP_cons]
      by_cases hp : p b'
      · rw [if_pos hp, if_pos (h a b' hfa hp)]
        omega
      · rw [if_neg hp]
        omega


theorem countP_map_le_count {α β : Type _} [DecidableEq α]
    [DecidableEq β] (f : α → β) (p : β → Prop) [DecidablePred p] (b : α)
    (h : ∀ a, p (f a) → a = b) (s : Multiset α) :
    (s.map f).countP p ≤ s.count b := by
  induction s using Multiset.induction_on with
  | empty => rfl
  | cons a s ih =>
    rw [Multiset.map_cons, Multiset.countP_cons, Multiset.count_cons]
    by_cases hp : p (f a)
    · rw [if_pos hp, if_pos (h a hp).symm]
      omega
    · rw [if_neg hp]
      omega


theorem countP_map_le_countP {α β : Type _} [DecidableEq α]
    [DecidableEq β] (f : α → β) (p : β → Prop) [DecidablePred p]
    (q : α → Prop) [DecidablePred q]
    (h : ∀ a, p (f a) → q a) (s : Multiset α) :
    (s.map f).countP p ≤ s.countP q := by
  induction s using Multiset.induction_on with
  | empty => rfl
  | cons a s ih =>
    rw [Multiset.map_cons, Multiset.countP_cons, Multiset.countP_cons]
    by_cases hp : p (f a)
    · rw [if_pos hp, if_pos (h a hp)]
      omega
    · rw [if_neg hp]
      by_cases hq : q a
      · rw [if_pos hq]
        omega
      · rw [if_neg hq]
        omega

theorem sum_map_zip_le {α β : Type _} (g : α × β → Nat)
    (h : α → Nat) (hg : ∀ x, g x ≤ h x.1) :
    ∀ (l : List α) (l' : List β),
      ((l.zip l').map g).sum ≤ (l.map h).sum
  | [], _ => Nat.zero_le _
  | _ :: _, [] => Nat.zero_le _
  | a :: l, b :: l' => by
    rw [List.zip_cons_cons, List.map_cons, List.map_cons, List.sum_cons,
      List.sum_cons]
    exact Nat.add_le_add (hg (a, b)) (sum_map_zip_le g h hg l l')



/-! ## Sampling a live latest value (`sample_every`'s read function) -/

/-- Read the live latest value at sampled tick indices, skipping empty
reads (`.latest()` of an `Optional` not yet present) and **blocking** at
the first unrealized tick: samples of the future wait, so under a
surrounding `fix` the sample stream stabilizes by prefix. -/
def sampleAtOpt {α : Type _} (tr : Trace (Option α)) : List Nat → List α
  | [] => []
  | u :: us =>
    match tr[u]? with
    | some (some x) => x :: sampleAtOpt tr us
    | some none => sampleAtOpt tr us
    | none => []

theorem sampleAtOpt_cons {α : Type _} (tr : Trace (Option α)) (u : Nat)
    (us : List Nat) :
    sampleAtOpt tr (u :: us)
      = match tr[u]? with
        | some (some x) => x :: sampleAtOpt tr us
        | some none => sampleAtOpt tr us
        | none => [] := rfl

/-- Sample reads extend under trajectory growth (decision fixed). -/
theorem sampleAtOpt_prefix {α : Type _} {tr tr' : Trace (Option α)}
    (h : tr <+: tr') :
    ∀ (idx : List Nat), sampleAtOpt tr idx <+: sampleAtOpt tr' idx
  | [] => List.prefix_refl _
  | u :: us => by
    rw [sampleAtOpt_cons, sampleAtOpt_cons]
    cases htr : tr[u]? with
    | none => exact List.nil_prefix
    | some ox =>
      obtain ⟨hu, -⟩ := List.getElem?_eq_some_iff.mp htr
      obtain ⟨e, rfl⟩ := h
      rw [List.getElem?_append_left hu, htr]
      cases ox with
      | some x =>
        exact List.cons_prefix_cons.mpr
          ⟨rfl, sampleAtOpt_prefix ⟨e, rfl⟩ us⟩
      | none => exact sampleAtOpt_prefix ⟨e, rfl⟩ us

/-- Every sample was a realized latest value. -/
theorem sampleAtOpt_mem {α : Type _} {tr : Trace (Option α)} {x : α} :
    ∀ {idx : List Nat}, x ∈ sampleAtOpt tr idx → some x ∈ tr
  | [], h => absurd h (List.not_mem_nil)
  | u :: us, h => by
    rw [sampleAtOpt_cons] at h
    revert h
    cases htr : tr[u]? with
    | none => exact fun h => absurd h (List.not_mem_nil)
    | some ox =>
      cases ox with
      | some y =>
        intro h
        rcases List.mem_cons.mp h with rfl | h'
        · exact List.mem_of_getElem? htr
        · exact sampleAtOpt_mem h'
      | none => exact fun h => sampleAtOpt_mem h

end HydroV2
