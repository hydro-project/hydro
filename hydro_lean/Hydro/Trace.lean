import Hydro.Grades

/-!
# Hydro · tick traces and cut legality

The realized-run vocabulary the interpretations share:

- `Trace σ` — a tick-located value's realized values **across all
  ticks** (one per realized tick); a batch stream is simply
  `Trace (List α)`.
- `foldAcrossTicksTrace`/`scanAcrossTicksTrace` — the cross-tick state fold (Rust `use::state`)
  and its emitting form (state + per-tick emission).
- `Ascending vo` — trajectory ascent of a trace in a value order: the
  pure content of a module's cross-tick ascent `ensures` face (a tick
  collection is always `Bounded` in Rust, so ascent is a contract fact,
  not a carrier grade).
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
| `use::state` register (fold) | `fold_across_ticks` | `foldAcrossTicksTrace` |
| `use::state` loop (emitting) | `tick_scan` (the `tick` block) | `scanAcrossTicksTrace` |
| `.batch(&tick, nondet!)` | `batch` | `batchCuts` |
| `.snapshot(&tick, nondet!)` | `snapshot` | `prefixCuts`/`snapshotCuts` |
| `.assume_ordering(nondet!)` | `assume_ordering` | `selectOrder` |
| `forward_ref` cycle | `fix_stream`/`fix_tick` | `iterate` |
-/

namespace Hydro

/-- A tick-scoped value's realized trace. -/
abbrev Trace (σ : Type _) : Type _ := List σ

/-- Same-tick pairing (tick atomicity: both wires read in one tick;
truncates to the jointly realized ticks). -/
@[reducible] def Trace.zip {α β : Type _} (a : Trace α) (b : Trace β) :
    Trace (α × β) :=
  List.zip a b

/-- Flattening preserves prefixes of emission traces. -/
theorem prefix_flatten {α : Type _} {a b : List (List α)}
    (h : a <+: b) : a.flatten <+: b.flatten := by
  obtain ⟨e, rfl⟩ := h
  exact ⟨e.flatten, (List.flatten_append (L₁ := a) (L₂ := e)).symm⟩

/-- **Tick-read eliminator** (indexed): position `u` of a zipped-mapped
tick wire is `f` of the two source reads at `u` — with the source
bounds surfaced. Kills the per-module `length`/`getElem` transport
around every `zipTick`+`mapTick` observation. -/
theorem Trace.zip_map_getElem {α β γ : Type _} {a : Trace α}
    {b : Trace β} {f : α × β → γ} {u : Nat}
    (hu : u < ((Trace.zip a b).map f).length) :
    ∃ (ha : u < a.length) (hb : u < b.length),
      ((Trace.zip a b).map f)[u]'hu = f (a[u]'ha, b[u]'hb) := by
  have hz : u < (Trace.zip a b).length := by rwa [List.length_map] at hu
  have hlen := hz
  simp only [Trace.zip, List.length_zip] at hlen
  refine ⟨by omega, by omega, ?_⟩
  rw [List.getElem_map]
  simp only [Trace.zip]
  rw [List.getElem_zip]

/-- **Tick-read eliminator** (membership): an element of a
zipped-mapped tick wire is `f` of the two source reads at some
jointly realized tick. -/
theorem Trace.mem_zip_map {α β γ : Type _} {a : Trace α} {b : Trace β}
    {f : α × β → γ} {g : γ} (hg : g ∈ (Trace.zip a b).map f) :
    ∃ (u : Nat) (ha : u < a.length) (hb : u < b.length),
      g = f (a[u]'ha, b[u]'hb) := by
  obtain ⟨u, hu, hgu⟩ := List.mem_iff_getElem.mp hg
  obtain ⟨ha, hb, he⟩ := Trace.zip_map_getElem hu
  exact ⟨u, ha, hb, by rw [← hgu, he]⟩

/-- **Tick-read eliminator** (plain zip, indexed): position `u` of a
zipped tick wire is the pair of the source reads at `u`. -/
theorem Trace.zip_getElem {α β : Type _} {a : Trace α} {b : Trace β}
    {u : Nat} (hu : u < (Trace.zip a b).length) :
    ∃ (ha : u < a.length) (hb : u < b.length),
      (Trace.zip a b)[u]'hu = (a[u]'ha, b[u]'hb) := by
  have hlen := hu
  simp only [Trace.zip, List.length_zip] at hlen
  refine ⟨by omega, by omega, ?_⟩
  simp only [Trace.zip]
  rw [List.getElem_zip]

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

/-- A stateless `tick` (unit register) is a per-tick map. -/
theorem scanAcrossTicksTrace_stateless {α γ : Type _} (f : α → γ) :
    ∀ (l : List α),
      scanAcrossTicksTrace (fun (_ : Unit) a => ((), f a)) () l = l.map f
  | [] => rfl
  | x :: xs => by
    show f x :: scanAcrossTicksTrace _ () xs = f x :: xs.map f
    rw [scanAcrossTicksTrace_stateless f xs]

/-- The register after a `use::state` run (the loop's final state). -/
def scanAcrossTicksState {ι σ β : Type _} (g : σ → ι → σ × β) :
    σ → List ι → σ
  | s, [] => s
  | s, x :: xs => scanAcrossTicksState g (g s x).1 xs

/-- **The `tick` construct's loop invariant** (Verus `while`-invariant
normal form): `P (emissions so far) (register)` holds at entry and is
preserved by one tick (the new emission appended, the register
stepped; `hlen` ties the accumulator to the tick index so positional
clauses can index the inputs) — then it holds of the whole run and
the final register. -/
theorem scanAcrossTicks_invariant {ι σ β : Type _} (g : σ → ι → σ × β)
    (P : List β → σ → Prop) (l : List ι) (seed : σ)
    (hinit : P [] seed)
    (htick : ∀ (n : Nat) (hn : n < l.length) (out : List β) (st : σ),
      out.length = n → P out st →
      P (out ++ [(g st (l[n]'hn)).2]) (g st (l[n]'hn)).1) :
    P (scanAcrossTicksTrace g seed l)
      (scanAcrossTicksState g seed l) := by
  -- generalized: walk the remaining suffix, tick index = |out|
  suffices h : ∀ (suf : List ι) (k : Nat) (out : List β) (st : σ),
      l.drop k = suf → out.length = k → P out st →
      P (out ++ scanAcrossTicksTrace g st suf)
        (scanAcrossTicksState g st suf) by
    simpa using h l 0 [] seed rfl rfl hinit
  intro suf
  induction suf with
  | nil =>
    intro k out st _ _ hP
    simpa [scanAcrossTicksTrace, scanAcrossTicksState] using hP
  | cons x rest ih =>
    intro k out st hdrop hlen hP
    have hkl : k < l.length := by
      have hl := congrArg List.length hdrop
      simp [List.length_drop] at hl
      omega
    have hcons := (List.drop_eq_getElem_cons hkl).symm.trans hdrop
    have hx : l[k]'hkl = x := (List.cons.injEq ..).mp hcons |>.1
    have hrest : l.drop (k + 1) = rest :=
      (List.cons.injEq ..).mp hcons |>.2
    have hstep := htick k hkl out st hlen hP
    rw [hx] at hstep
    rw [show scanAcrossTicksTrace g st (x :: rest)
      = (g st x).2 :: scanAcrossTicksTrace g (g st x).1 rest from rfl]
    rw [show scanAcrossTicksState g st (x :: rest)
      = scanAcrossTicksState g (g st x).1 rest from rfl]
    have := ih (k + 1) (out ++ [(g st x).2]) (g st x).1 hrest
      (by simp [hlen]) hstep
    simpa [List.append_assoc] using this

/-- The run's prefix is the run over the input's prefix (one tick per
input: cutting the inputs cuts the run). -/
theorem scanAcrossTicksTrace_take {ι σ β : Type _} (g : σ → ι → σ × β) :
    ∀ (s : σ) (l : List ι) (n : Nat),
      (scanAcrossTicksTrace g s l).take n
        = scanAcrossTicksTrace g s (l.take n)
  | _, [], _ => by simp [scanAcrossTicksTrace]
  | _, _ :: _, 0 => rfl
  | s, x :: xs, n + 1 => by
    show (g s x).2 :: (scanAcrossTicksTrace g (g s x).1 xs).take n
      = (g s x).2 :: scanAcrossTicksTrace g (g s x).1 (xs.take n)
    rw [scanAcrossTicksTrace_take g _ xs n]

/-- The register after the first `n + 1` ticks is the step on the
register after `n` ticks and the `n`-th input. -/
theorem scanAcrossTicksState_take_succ {ι σ β : Type _} (g : σ → ι → σ × β) :
    ∀ (s : σ) (l : List ι) (n : Nat) (hn : n < l.length),
      scanAcrossTicksState g s (l.take (n + 1))
        = (g (scanAcrossTicksState g s (l.take n)) (l[n]'hn)).1
  | _, [], _, hn => absurd hn (Nat.not_lt_zero _)
  | _, _ :: _, 0, _ => rfl
  | s, x :: xs, n + 1, hn =>
    scanAcrossTicksState_take_succ g (g s x).1 xs n (Nat.lt_of_succ_lt_succ hn)

/-- Tick `n`'s emission is the step on the register after `n` ticks and
the `n`-th input (reading a loop's output at one iteration). -/
theorem scanAcrossTicksTrace_getElem {ι σ β : Type _} (g : σ → ι → σ × β) :
    ∀ (s : σ) (l : List ι) (n : Nat) (hn : n < l.length),
      (scanAcrossTicksTrace g s l)[n]'(by rw [scanAcrossTicksTrace_length]; exact hn)
        = (g (scanAcrossTicksState g s (l.take n)) (l[n]'hn)).2
  | _, [], _, hn => absurd hn (Nat.not_lt_zero _)
  | _, _ :: _, 0, _ => rfl
  | s, x :: xs, n + 1, hn =>
    scanAcrossTicksTrace_getElem g (g s x).1 xs n (Nat.lt_of_succ_lt_succ hn)

/-- **The loop invariant at every iteration**: `P` holds of the run's
first `n` emissions and the register after `n` ticks (the invariant
`scanAcrossTicks_invariant` establishes at the end, read at a prefix —
how a face about one tick's emission cites the register that produced
it). -/
theorem scanAcrossTicks_invariant_take {ι σ β : Type _} (g : σ → ι → σ × β)
    (P : List β → σ → Prop) (l : List ι) (seed : σ)
    (hinit : P [] seed)
    (htick : ∀ (n : Nat) (hn : n < l.length) (out : List β) (st : σ),
      out.length = n → P out st →
      P (out ++ [(g st (l[n]'hn)).2]) (g st (l[n]'hn)).1)
    (n : Nat) :
    P ((scanAcrossTicksTrace g seed l).take n)
      (scanAcrossTicksState g seed (l.take n)) := by
  rw [scanAcrossTicksTrace_take]
  refine scanAcrossTicks_invariant g P (l.take n) seed hinit ?_
  intro k hk out st hlen hP
  have hk' : k < l.length := by
    have := hk
    rw [List.length_take] at this
    omega
  rw [List.getElem_take]
  exact htick k hk' out st hlen hP

/-! ## Reading tick loops without indices (D63)

The vocabulary a face or a loop-body proof should be written in — so
that what remains is the protocol argument, not `getElem` bound
transport:

- **option-indexed reads** (`l[n]? = some x`): a read at a tick needs no
  bound proof; the `[n]?` forms of `zip`/`map`/scan push through by
  `simp`;
- **the loop's history** `Trace.hist inp out` (inputs consumed so far,
  paired with their emissions): a loop invariant's cross-iteration
  clauses are MEMBERSHIP clauses (`∀ p ∈ hist, …`) and two-iteration
  clauses are `List.Pairwise` — then one tick appends one pair and the
  step obligation splits by the snoc simp set into "the old history
  (the induction hypothesis)" and "this tick's pair (the protocol
  fact)", with zero index arithmetic;
- **the option-indexed loop induction** (`scanAcrossTicks_invariant?`):
  the step is handed this tick's input by `l[n]? = some x`. -/

section TickReads

variable {ι σ β : Type _}

/-- `[n]?` through `Trace.zip`: a read of a zipped wire is a read of each
wire at the same tick. -/
theorem Trace.getElem?_zip_eq_some {α β : Type _} {a : Trace α} {b : Trace β}
    {n : Nat} {x : α} {y : β} :
    (Trace.zip a b)[n]? = some (x, y) ↔ a[n]? = some x ∧ b[n]? = some y :=
  List.getElem?_zip_eq_some

/-- `[n]?` through `Trace.zip`, the pair unnamed. -/
theorem Trace.getElem?_zip_eq_some' {α β : Type _} {a : Trace α} {b : Trace β}
    {n : Nat} {z : α × β} :
    (Trace.zip a b)[n]? = some z ↔ a[n]? = some z.1 ∧ b[n]? = some z.2 :=
  List.getElem?_zip_eq_some

/-- Tick `n`'s emission, option-indexed: the step on the register after
`n` ticks and tick `n`'s input. -/
theorem scanAcrossTicksTrace_getElem? (g : σ → ι → σ × β) (s : σ) (l : List ι)
    (n : Nat) :
    (scanAcrossTicksTrace g s l)[n]?
      = (l[n]?).map fun x => (g (scanAcrossTicksState g s (l.take n)) x).2 := by
  by_cases hn : n < l.length
  · rw [List.getElem?_eq_getElem hn,
      List.getElem?_eq_getElem (by rw [scanAcrossTicksTrace_length]; exact hn),
      scanAcrossTicksTrace_getElem g s l n hn]
    rfl
  · rw [List.getElem?_eq_none (Nat.le_of_not_lt hn),
      List.getElem?_eq_none (by rw [scanAcrossTicksTrace_length]; exact Nat.le_of_not_lt hn)]
    rfl

/-- Reading tick `n` of a loop: an emission `e` at `n` is the step on the
register before `n` and the input at `n`. -/
theorem scanAcrossTicksTrace_getElem?_eq_some (g : σ → ι → σ × β) (s : σ)
    (l : List ι) (n : Nat) (e : β) :
    (scanAcrossTicksTrace g s l)[n]? = some e ↔
      ∃ x, l[n]? = some x ∧ e = (g (scanAcrossTicksState g s (l.take n)) x).2 := by
  rw [scanAcrossTicksTrace_getElem?, Option.map_eq_some_iff]
  constructor
  · rintro ⟨x, hx, rfl⟩; exact ⟨x, hx, rfl⟩
  · rintro ⟨x, hx, rfl⟩; exact ⟨x, hx, rfl⟩

/-- **The loop's history**: the inputs consumed so far, each paired with
the emission it produced (one pair per iteration). -/
abbrev Trace.hist (inp : List ι) (out : List β) : List (ι × β) :=
  List.zip inp out

/-- One tick appends one pair to the history. -/
theorem Trace.hist_snoc :
    ∀ {inp : List ι} {out : List β} {n : Nat}
      (hn : n < inp.length) (_hlen : out.length = n) (e : β),
      Trace.hist inp (out ++ [e]) = Trace.hist inp out ++ [(inp[n]'hn, e)]
  | [], _, _, hn, _, _ => absurd hn (Nat.not_lt_zero _)
  | _ :: _, [], 0, _, _, _ => by simp [Trace.hist]
  | x :: xs, y :: ys, n + 1, hn, hlen, e => by
    have ih := Trace.hist_snoc (inp := xs) (out := ys) (n := n)
      (Nat.lt_of_succ_lt_succ hn) (Nat.succ.inj hlen) e
    simp only [Trace.hist] at ih ⊢
    simp only [List.cons_append, List.zip_cons_cons, List.getElem_cons_succ, ih]

/-- One tick appends one pair to the history (option-indexed input). -/
theorem Trace.hist_snoc? {inp : List ι} {out : List β} {n : Nat} {x : ι}
    (hx : inp[n]? = some x) (hlen : out.length = n) (e : β) :
    Trace.hist inp (out ++ [e]) = Trace.hist inp out ++ [(x, e)] := by
  obtain ⟨hn, rfl⟩ := List.getElem?_eq_some_iff.mp hx
  exact Trace.hist_snoc hn hlen e

/-- A history pair's input is an input. -/
theorem Trace.hist_mem_input {inp : List ι} {out : List β} {p : ι × β}
    (hp : p ∈ Trace.hist inp out) : p.1 ∈ inp :=
  (List.of_mem_zip hp).1

/-- A history pair is a tick: both reads at one index. -/
theorem Trace.mem_hist_iff {inp : List ι} {out : List β} {p : ι × β} :
    p ∈ Trace.hist inp out ↔ ∃ n : Nat, inp[n]? = some p.1 ∧ out[n]? = some p.2 := by
  rw [List.mem_iff_getElem?]
  simp only [Trace.hist, List.getElem?_zip_eq_some]

/-- Under a `Pairwise` fact on the inputs, every earlier history pair's
input relates to this tick's input. -/
theorem Trace.hist_mem_pairwise {R : ι → ι → Prop} {inp : List ι}
    {out : List β} {n : Nat} (hn : n < inp.length) (hlen : out.length = n)
    (hR : List.Pairwise R inp) {p : ι × β} (hp : p ∈ Trace.hist inp out) :
    R p.1 (inp[n]'hn) := by
  obtain ⟨k, hk, hkp⟩ := List.mem_iff_getElem.mp hp
  have hk' : k < out.length := by
    simp only [Trace.hist, List.length_zip] at hk; omega
  have hki : k < inp.length := by
    simp only [Trace.hist, List.length_zip] at hk; omega
  have hfst : p.1 = inp[k]'hki := by
    rw [← hkp]; simp only [Trace.hist]; rw [List.getElem_zip]
  rw [hfst]
  exact List.pairwise_iff_getElem.mp hR k n hki hn (by omega)

/-- Option-indexed form of `Trace.hist_mem_pairwise`. -/
theorem Trace.hist_mem_pairwise? {R : ι → ι → Prop} {inp : List ι}
    {out : List β} {n : Nat} {x : ι} (hx : inp[n]? = some x) (hlen : out.length = n)
    (hR : List.Pairwise R inp) {p : ι × β} (hp : p ∈ Trace.hist inp out) :
    R p.1 x := by
  obtain ⟨hn, rfl⟩ := List.getElem?_eq_some_iff.mp hx
  exact Trace.hist_mem_pairwise hn hlen hR hp

/-! ### The snoc simp set: one appended element, split off -/

theorem forall_mem_snoc {α : Type _} {l : List α} {a : α} {Q : α → Prop} :
    (∀ x ∈ l ++ [a], Q x) ↔ (∀ x ∈ l, Q x) ∧ Q a := by
  simp only [List.mem_append, List.mem_singleton]
  constructor
  · intro h; exact ⟨fun x hx => h x (Or.inl hx), h a (Or.inr rfl)⟩
  · rintro ⟨h1, h2⟩ x (hx | rfl); exacts [h1 x hx, h2]

theorem exists_mem_snoc {α : Type _} {l : List α} {a : α} {Q : α → Prop} :
    (∃ x ∈ l ++ [a], Q x) ↔ (∃ x ∈ l, Q x) ∨ Q a := by
  simp only [List.mem_append, List.mem_singleton]
  constructor
  · rintro ⟨x, hx | rfl, hq⟩; exacts [Or.inl ⟨x, hx, hq⟩, Or.inr hq]
  · rintro (⟨x, hx, hq⟩ | hq); exacts [⟨x, Or.inl hx, hq⟩, ⟨a, Or.inr rfl, hq⟩]

theorem pairwise_snoc {α : Type _} {R : α → α → Prop} {l : List α} {a : α} :
    List.Pairwise R (l ++ [a]) ↔ List.Pairwise R l ∧ ∀ x ∈ l, R x a := by
  rw [List.pairwise_append]
  simp only [List.pairwise_singleton, List.mem_singleton, true_and, forall_eq]

/-! ### The loop induction, option-indexed -/

/-- `scanAcrossTicks_invariant` with the step handed tick `n`'s input by
an option read (`l[n]? = some x`): the loop-body obligation the `tick`
construct states. -/
theorem scanAcrossTicks_invariant? (g : σ → ι → σ × β)
    (P : List β → σ → Prop) (l : List ι) (seed : σ)
    (hinit : P [] seed)
    (htick : ∀ (n : Nat) (out : List β) (st : σ) (x : ι),
      l[n]? = some x → out.length = n → P out st →
      P (out ++ [(g st x).2]) (g st x).1) :
    P (scanAcrossTicksTrace g seed l) (scanAcrossTicksState g seed l) :=
  scanAcrossTicks_invariant g P l seed hinit
    (fun n hn out st hlen hP =>
      htick n out st (l[n]'hn) (List.getElem?_eq_getElem hn) hlen hP)

/-- `scanAcrossTicks_invariant_take`, option-indexed. -/
theorem scanAcrossTicks_invariant_take? (g : σ → ι → σ × β)
    (P : List β → σ → Prop) (l : List ι) (seed : σ)
    (hinit : P [] seed)
    (htick : ∀ (n : Nat) (out : List β) (st : σ) (x : ι),
      l[n]? = some x → out.length = n → P out st →
      P (out ++ [(g st x).2]) (g st x).1) (n : Nat) :
    P ((scanAcrossTicksTrace g seed l).take n)
      (scanAcrossTicksState g seed (l.take n)) :=
  scanAcrossTicks_invariant_take g P l seed hinit
    (fun n hn out st hlen hP =>
      htick n out st (l[n]'hn) (List.getElem?_eq_getElem hn) hlen hP) n

/-- The register before tick `n + 1` is the step on the register before
tick `n` and tick `n`'s input (option-indexed). -/
theorem scanAcrossTicksState_take_succ? (g : σ → ι → σ × β) (s : σ) (l : List ι)
    {n : Nat} {x : ι} (hx : l[n]? = some x) :
    scanAcrossTicksState g s (l.take (n + 1))
      = (g (scanAcrossTicksState g s (l.take n)) x).1 := by
  obtain ⟨hn, rfl⟩ := List.getElem?_eq_some_iff.mp hx
  exact scanAcrossTicksState_take_succ g s l n hn

/-- Once the inputs end, the register before tick `n + 1` is the register
before tick `n`. -/
theorem scanAcrossTicksState_take_stall (g : σ → ι → σ × β) (s : σ) (l : List ι)
    {n : Nat} (hn : l[n]? = none) :
    scanAcrossTicksState g s (l.take (n + 1)) = scanAcrossTicksState g s (l.take n) := by
  have hge : l.length ≤ n := List.getElem?_eq_none_iff.mp hn
  rw [List.take_of_length_le hge, List.take_of_length_le (Nat.le_succ_of_le hge)]

/-- A zip's read is `none` once either side has ended. -/
theorem Trace.getElem?_zip_eq_none_right {α β : Type _} {a : Trace α} {b : Trace β}
    {n : Nat} (hb : b[n]? = none) : (Trace.zip a b)[n]? = none := by
  rw [List.getElem?_eq_none_iff] at hb ⊢
  simp only [Trace.zip, List.length_zip]; omega

theorem Trace.getElem?_zip_eq_none_left {α β : Type _} {a : Trace α} {b : Trace β}
    {n : Nat} (ha : a[n]? = none) : (Trace.zip a b)[n]? = none := by
  rw [List.getElem?_eq_none_iff] at ha ⊢
  simp only [Trace.zip, List.length_zip]; omega

/-- **A register fact preserved by every step holds before every tick**
(the register-only loop invariant, read at every prefix — for blocks the
`invariant` clause does not cover, e.g. persisted-stream states). -/
theorem scanAcrossTicksState_induct (g : σ → ι → σ × β) (seed : σ) (l : List ι)
    (Q : σ → Prop) (hseed : Q seed)
    (hstep : ∀ st x, x ∈ l → Q st → Q (g st x).1) :
    ∀ n, Q (scanAcrossTicksState g seed (l.take n))
  | 0 => hseed
  | n + 1 => by
    cases hx : l[n]? with
    | none =>
      have hge : l.length ≤ n := List.getElem?_eq_none_iff.mp hx
      rw [List.take_of_length_le (by omega)]
      have := scanAcrossTicksState_induct g seed l Q hseed hstep n
      rwa [List.take_of_length_le hge] at this
    | some x =>
      rw [scanAcrossTicksState_take_succ? g seed l hx]
      exact hstep _ x (List.mem_iff_getElem?.mpr ⟨n, hx⟩)
        (scanAcrossTicksState_induct g seed l Q hseed hstep n)

/-- **Chaining a per-step register fact along a run**: if every tick
whose input satisfies `cond` steps the register along a transitive
relation `R`, then across any stretch of ticks all satisfying `cond` the
registers before and after are `R`-related (the "the register never
decreases while no rebase happens" shape — one lemma instead of a
history-indexed invariant clause). -/
theorem scanAcrossTicksState_chain (g : σ → ι → σ × β) (seed : σ) (l : List ι)
    (R : σ → σ → Prop) (hrefl : ∀ s, R s s) (htrans : ∀ a b c, R a b → R b c → R a c)
    (cond : ι → Prop) (hstep : ∀ st x, cond x → R st (g st x).1) :
    ∀ (u k : Nat), (∀ w x, u ≤ w → w < u + k → l[w]? = some x → cond x) →
      R (scanAcrossTicksState g seed (l.take u))
        (scanAcrossTicksState g seed (l.take (u + k)))
  | u, 0, _ => hrefl _
  | u, k + 1, hc => by
    have ih := scanAcrossTicksState_chain g seed l R hrefl htrans cond hstep u k
      (fun w x hw hw' hx => hc w x hw (by omega) hx)
    refine htrans _ _ _ ih ?_
    cases hx : l[u + k]? with
    | none =>
      have hge : l.length ≤ u + k := List.getElem?_eq_none_iff.mp hx
      rw [List.take_of_length_le hge, List.take_of_length_le (by omega)]
      exact hrefl _
    | some x =>
      rw [show u + (k + 1) = u + k + 1 by omega, scanAcrossTicksState_take_succ? g seed l hx]
      exact hstep _ x (hc _ x (by omega) (by omega) hx)

/-! ### The first tick where a predicate holds -/

/-- **The first tick satisfying `p`**, given one does: a witness index
`n₀` with its read, no earlier read satisfying `p`, and `n₀` below the
given one. The "first leader tick of a reign" shape. -/
theorem Trace.first_tick {α : Type _} (p : α → Bool) {l : Trace α} {n : Nat} {x : α}
    (hx : l[n]? = some x) (hp : p x = true) :
    ∃ (n₀ : Nat) (x₀ : α), n₀ ≤ n ∧ l[n₀]? = some x₀ ∧ p x₀ = true
      ∧ ∀ (m : Nat) (y : α), m < n₀ → l[m]? = some y → p y = false := by
  obtain ⟨hn, rfl⟩ := List.getElem?_eq_some_iff.mp hx
  have hsome : ∃ k, l.findIdx? p = some k := by
    cases h : l.findIdx? p with
    | some k => exact ⟨k, rfl⟩
    | none =>
      exact absurd hp (by
        have := List.findIdx?_eq_none_iff.mp h _ (List.getElem_mem hn)
        rw [this]; exact Bool.false_ne_true)
  obtain ⟨k, hk⟩ := hsome
  obtain ⟨hkl, hpk, hmin⟩ := List.findIdx?_eq_some_iff_getElem.mp hk
  refine ⟨k, l[k]'hkl, ?_, List.getElem?_eq_getElem hkl, hpk, ?_⟩
  · by_contra hlt
    exact hmin n (Nat.lt_of_not_le hlt) hp
  · intro m y hm hy
    obtain ⟨hml, rfl⟩ := List.getElem?_eq_some_iff.mp hy
    cases h : p (l[m]'hml)
    · rfl
    · exact absurd h (hmin m hm)

/-- Between two reads of a `Pairwise`-ordered trace, the earlier is
related to the later. -/
theorem Trace.pairwise_reads {α : Type _} {R : α → α → Prop} {l : Trace α}
    (hR : List.Pairwise R l) {m n : Nat} {x y : α} (hmn : m < n)
    (hx : l[m]? = some x) (hy : l[n]? = some y) : R x y := by
  obtain ⟨hm, rfl⟩ := List.getElem?_eq_some_iff.mp hx
  obtain ⟨hn, rfl⟩ := List.getElem?_eq_some_iff.mp hy
  exact List.pairwise_iff_getElem.mp hR m n hm hn hmn

/-- A `Pairwise` fact on a wire lifts to any wire whose reads project onto
it (a zipped tuple, read componentwise). -/
theorem Trace.pairwise_of_reads {α γ : Type _} {R : α → α → Prop} {a : Trace α}
    {l : Trace γ} (f : γ → α)
    (hf : ∀ (n : Nat) (x : γ), l[n]? = some x → a[n]? = some (f x))
    (hR : List.Pairwise R a) : List.Pairwise (fun x y => R (f x) (f y)) l := by
  rw [List.pairwise_iff_getElem]
  intro m n hm hn hmn
  exact Trace.pairwise_reads hR hmn (hf m _ (List.getElem?_eq_getElem hm))
    (hf n _ (List.getElem?_eq_getElem hn))

/-- A read is a member. -/
theorem Trace.mem_of_read {α : Type _} {l : Trace α} {n : Nat} {x : α}
    (hx : l[n]? = some x) : x ∈ l :=
  List.mem_iff_getElem?.mpr ⟨n, hx⟩

/-- Two reads at one tick agree. -/
theorem Trace.read_inj {α : Type _} {l : Trace α} {n : Nat} {x y : α}
    (hx : l[n]? = some x) (hy : l[n]? = some y) : x = y :=
  Option.some.inj (hx.symm.trans hy)

/-- A read at `n` is below the length. -/
theorem Trace.read_lt {α : Type _} {l : Trace α} {n : Nat} {x : α}
    (hx : l[n]? = some x) : n < l.length :=
  (List.getElem?_eq_some_iff.mp hx).1

/-- `[n]?` through `List.map`, `some`-shaped. -/
theorem Trace.getElem?_map_eq_some {α β : Type _} {f : α → β} {l : Trace α}
    {n : Nat} {y : β} :
    (l.map f)[n]? = some y ↔ ∃ x, l[n]? = some x ∧ y = f x := by
  rw [List.getElem?_map, Option.map_eq_some_iff]
  constructor
  · rintro ⟨x, hx, rfl⟩; exact ⟨x, hx, rfl⟩
  · rintro ⟨x, hx, rfl⟩; exact ⟨x, hx, rfl⟩

/-- `[n]?` through `defer_tick`'s `cons`: tick `0` reads the seed, tick
`n + 1` reads tick `n`. -/
theorem Trace.getElem?_cons_succ' {α : Type _} (v : α) (l : Trace α) (n : Nat) :
    (v :: l)[n + 1]? = l[n]? := rfl

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

/-- **At most one across ticks**: the sum of a trace of batches holds at
most one `p`-element when every batch holds at most one and no two
distinct ticks both hold one. -/
theorem Trace.sum_countP_le_one {α : Type _} (p : α → Prop) [DecidablePred p] :
    ∀ (L : Trace (Multiset α)),
    (∀ (t : Nat) (e : Multiset α), L[t]? = some e → e.countP p ≤ 1) →
    (∀ (t t' : Nat) (e e' : Multiset α), t < t' → L[t]? = some e → L[t']? = some e' →
      ∀ x ∈ e, ∀ y ∈ e', p x → p y → False) →
    L.sum.countP p ≤ 1
  | [], _, _ => by simp
  | e :: L, hin, hacross => by
    rw [List.sum_cons, Multiset.countP_add]
    have hrest := Trace.sum_countP_le_one p L
      (fun t e' h => hin (t + 1) e' h)
      (fun t t' e₁ e₂ h h1 h2 => hacross (t + 1) (t' + 1) e₁ e₂ (by omega) h1 h2)
    have he := hin 0 e rfl
    by_cases h0 : e.countP p = 0
    · omega
    · obtain ⟨x, hx, hpx⟩ := Multiset.countP_pos.mp (Nat.pos_of_ne_zero h0)
      have hz : L.sum.countP p = 0 := by
        rw [Multiset.countP_eq_zero]
        intro y hy hpy
        obtain ⟨e', he', hye⟩ := mem_list_sum.mp hy
        obtain ⟨t, ht⟩ := List.mem_iff_getElem?.mp he'
        exact hacross 0 (t + 1) e e' (by omega) rfl ht x hx y hye hpx hpy
      omega


/-! ### Generic list facts the per-tick protocol ghosts cite -/

/-- A list containing `x` has a maximum, at or above `x`. -/
theorem List.nat_max?_ge : ∀ (l : List Nat) {x : Nat}, x ∈ l →
    ∃ m, l.max? = some m ∧ x ≤ m
  | [], _, h => nomatch h
  | y :: ys, x, h => by
    rcases List.mem_cons.mp h with rfl | h'
    · cases hys : ys.max? with
      | none =>
        exact ⟨x, by rw [List.max?_cons, hys]; rfl, Nat.le_refl _⟩
      | some m =>
        exact ⟨max x m, by rw [List.max?_cons, hys]; rfl,
          Nat.le_max_left ..⟩
    · obtain ⟨m, hm, hx⟩ := List.nat_max?_ge ys h'
      refine ⟨max y m, by rw [List.max?_cons, hm]; rfl, ?_⟩
      exact Nat.le_trans hx (Nat.le_max_right ..)

/-- `filterMap`s that preserve a projection keep it a sublist. -/
theorem List.map_filterMap_sublist {α β γ : Type _}
    (fn : α → Option β) (g : β → γ) (g' : α → γ)
    (h : ∀ a b, fn a = some b → g b = g' a) :
    ∀ (l : List α), ((l.filterMap fn).map g).Sublist (l.map g')
  | [] => List.Sublist.refl _
  | a :: l => by
    rw [List.filterMap_cons]
    cases hfa : fn a with
    | none =>
      rw [List.map_cons]
      exact (List.map_filterMap_sublist fn g g' h l).cons _
    | some b =>
      rw [List.map_cons, List.map_cons, h a b hfa]
      exact (List.map_filterMap_sublist fn g g' h l).cons_cons _

/-- In a key-duplicate-free association list, a key has one value. -/
theorem List.eq_of_keys_nodup {α κ : Type _} {l : List (κ × α)}
    (hnd : (l.map Prod.fst).Nodup) {k : κ} {a a' : α}
    (h : (k, a) ∈ l) (h' : (k, a') ∈ l) : a = a' := by
  induction l with
  | nil => cases h
  | cons x xs ih =>
    rw [List.map_cons] at hnd
    rcases List.mem_cons.mp h with rfl | h2
    · rcases List.mem_cons.mp h' with h1' | h2'
      · exact (congrArg Prod.snd h1'.symm :)
      · exfalso
        exact (List.nodup_cons.mp hnd).1
          (List.mem_map.mpr ⟨(k, a'), h2', rfl⟩)
    · rcases List.mem_cons.mp h' with rfl | h2'
      · exfalso
        exact (List.nodup_cons.mp hnd).1
          (List.mem_map.mpr ⟨(k, a), h2, rfl⟩)
      · exact ih (List.nodup_cons.mp hnd).2 h2 h2'

end TickReads

/-! ## Generic scan-run combinators (`use::state` induction schemes)

A module proves small obligations about **its own step function** and
instantiates these; the run-level induction boilerplate lives here,
once, for every `use::state` loop:

- `scan_sound` — per-emission provenance: every run emission satisfies
  a pool-monotone predicate of the consumed pool;
- `scan_emit_ind` — emission-accumulator induction with an ambient
  total (the shape of capped/crossing arguments: the invariant may use
  `prefix ≤ total` to import usage-contract caps);
- `scan_bound` — the potential-function (amortized) bound: per-tick
  emissions plus the next potential bounded by the current potential
  plus the tick's contribution give the run bound, unconditionally. -/

section ScanRun

variable {ι σ : Type _} {α γ δ : Type _}

/-- **Per-emission provenance**: if the step's carried window `W`
stays inside its consumed inputs and every tick's emission satisfies a
pool-monotone predicate `Q` of its window, every run emission
satisfies `Q` of the whole consumed pool. -/
theorem scan_sound (g : σ → ι → σ × Multiset γ) (addC : ι → Multiset α)
    (W : σ → Multiset α) (Q : Multiset α → γ → Prop)
    (hQmono : ∀ {p p' : Multiset α} {y : γ}, p ≤ p' → Q p y → Q p' y)
    (hW : ∀ s b, W (g s b).1 ≤ W s + addC b)
    (hemit : ∀ s b {y : γ}, y ∈ (g s b).2 → Q (W s + addC b) y)
    (bs : List ι) :
    ∀ (s : σ) (pfx : Multiset α), W s ≤ pfx →
      ∀ {y : γ}, y ∈ (scanAcrossTicksTrace g s bs).sum →
        Q (pfx + (bs.map addC).sum) y := by
  induction bs with
  | nil =>
    intro s pfx _ y h
    rw [show scanAcrossTicksTrace g s [] = [] from rfl,
      List.sum_nil] at h
    exact absurd h (Multiset.notMem_zero y)
  | cons b bs ih =>
    intro s pfx hw y h
    rw [show scanAcrossTicksTrace g s (b :: bs)
        = (g s b).2 :: scanAcrossTicksTrace g (g s b).1 bs from rfl,
      List.sum_cons] at h
    have hsum : pfx + ((b :: bs).map addC).sum
        = (pfx + addC b) + (bs.map addC).sum := by
      rw [List.map_cons, List.sum_cons, ← Multiset.add_assoc]
    rcases Multiset.mem_add.mp h with hhd | htl
    · refine hQmono ?_ (hemit s b hhd)
      rw [hsum]
      exact le_trans (Multiset.add_le_add_right hw)
        (Multiset.le_add_right ..)
    · rw [hsum]
      exact ih (g s b).1 (pfx + addC b)
        (le_trans (hW s b) (Multiset.add_le_add_right hw)) htl

/-- **Emission-accumulator induction with an ambient total**: an
invariant over (state, consumed prefix) preserved by the step below
the total, a base case at the exhausted prefix, and a glue absorbing
each tick's emission into a prefix-indexed goal give the goal over the
whole run. (The `≤ total` inputs are how usage-contract caps reach the
step obligations.) -/
theorem scan_emit_ind (g : σ → ι → σ × Multiset γ)
    (addC : ι → Multiset α) (total : Multiset α)
    (Inv : σ → Multiset α → Prop)
    (Goal : Multiset α → Multiset γ → Prop)
    (hstep : ∀ s pfx b, pfx + addC b ≤ total → Inv s pfx →
        Inv (g s b).1 (pfx + addC b))
    (hbase : Goal total 0)
    (hglue : ∀ s pfx b acc, pfx + addC b ≤ total → Inv s pfx →
        Goal (pfx + addC b) acc → Goal pfx ((g s b).2 + acc))
    (bs : List ι) :
    ∀ (s : σ) (pfx : Multiset α),
      pfx + (bs.map addC).sum = total → Inv s pfx →
      Goal pfx (scanAcrossTicksTrace g s bs).sum := by
  induction bs with
  | nil =>
    intro s pfx htot _
    rw [List.map_nil, List.sum_nil, Multiset.add_zero] at htot
    subst htot
    rw [show scanAcrossTicksTrace g s [] = [] from rfl, List.sum_nil]
    exact hbase
  | cons b bs ih =>
    intro s pfx htot hinv
    have htot' : (pfx + addC b) + (bs.map addC).sum = total := by
      rw [Multiset.add_assoc, ← List.sum_cons, ← List.map_cons]
      exact htot
    have hle : pfx + addC b ≤ total := by
      rw [← htot']
      exact Multiset.le_add_right ..
    rw [show scanAcrossTicksTrace g s (b :: bs)
        = (g s b).2 :: scanAcrossTicksTrace g (g s b).1 bs from rfl,
      List.sum_cons]
    exact hglue s pfx b _ hle hinv
      (ih (g s b).1 (pfx + addC b) htot' (hstep s pfx b hle hinv))

/-- **The potential-function bound** (amortized, unconditional): if,
under an entry invariant, each tick's projected emission plus the next
potential is bounded by the current potential plus the tick's
contribution, then the run's projected emission is bounded by the
initial potential plus the total contribution. -/
theorem scan_bound (g : σ → ι → σ × Multiset γ)
    (p : Multiset γ → Multiset δ) (hp0 : p 0 = 0)
    (hp : ∀ x y, p (x + y) = p x + p y)
    (addC : ι → Multiset δ) (Inv : σ → Prop) (B : σ → Multiset δ)
    (hstep : ∀ s b, Inv s →
        Inv (g s b).1 ∧ p (g s b).2 + B (g s b).1 ≤ B s + addC b)
    (bs : List ι) :
    ∀ (s : σ), Inv s →
      p (scanAcrossTicksTrace g s bs).sum
        ≤ B s + (bs.map addC).sum := by
  induction bs with
  | nil =>
    intro s _
    rw [show scanAcrossTicksTrace g s [] = [] from rfl, List.sum_nil,
      hp0]
    exact Multiset.zero_le _
  | cons b bs ih =>
    intro s hinv
    obtain ⟨hinv', hstep'⟩ := hstep s b hinv
    rw [show scanAcrossTicksTrace g s (b :: bs)
        = (g s b).2 :: scanAcrossTicksTrace g (g s b).1 bs from rfl,
      List.sum_cons, hp, List.map_cons, List.sum_cons]
    calc p (g s b).2 + p (scanAcrossTicksTrace g (g s b).1 bs).sum
        ≤ p (g s b).2 + (B (g s b).1 + (bs.map addC).sum) :=
          Multiset.add_le_add_left (ih (g s b).1 hinv')
      _ = (p (g s b).2 + B (g s b).1) + (bs.map addC).sum := by
          rw [Multiset.add_assoc]
      _ ≤ (B s + addC b) + (bs.map addC).sum :=
          Multiset.add_le_add_right hstep'
      _ = B s + (addC b + (bs.map addC).sum) := by
          rw [Multiset.add_assoc]

end ScanRun

/-! ## Trajectory-ascending traces (the pure content of ascent faces) -/

/-- Ascent along the realized trajectory in a value order. -/
def Ascending {σ : Type _} (vo : ValueOrder σ) (l : Trace σ) : Prop :=
  ∀ {t t' : Nat} (h : t ≤ t') (ht' : t' < l.length),
    vo.le (l[t]'(Nat.lt_of_le_of_lt h ht')) (l[t']'ht')

/-- Order-preserving image of an ascending trace (Rust
`SingletonMapFuncAlgebra`'s `order_preserving`). -/
theorem Ascending.map {σ τ : Type _} {vo : ValueOrder σ}
    {vo' : ValueOrder τ} {l : Trace σ} (hl : Ascending vo l) (h : σ → τ)
    (hpres : ∀ {a b}, vo.le a b → vo'.le (h a) (h b)) :
    Ascending vo' (l.map h) := by
  intro t t' hle ht'
  have hl' : t' < l.length := by
    have := ht'
    rwa [List.length_map] at this
  rw [List.getElem_map, List.getElem_map]
  exact hpres (hl hle hl')

/-- The running fold of an inflationary step ascends along the ticks —
the pure content of a `use::state` accumulator's `ensures` ascent face
(cross-tick ascent is a contract fact, not a carrier grade). -/
theorem foldAcrossTicksTrace_ascending {ι σ : Type _} (vo : ValueOrder σ)
    (g : σ → ι → σ) (init : σ) (hinfl : ∀ s x, vo.le s (g s x))
    (xs : List ι) : Ascending vo (foldAcrossTicksTrace g init xs) := by
  intro t t' h ht'
  have ht : t < (foldAcrossTicksTrace g init xs).length := Nat.lt_of_le_of_lt h ht'
  rw [foldAcrossTicksTrace_getElem g init xs t ht,
    foldAcrossTicksTrace_getElem g init xs t' ht']
  exact vo.foldl_take_le hinfl (Nat.succ_le_succ h) init

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

/-- A prefix-cut read (option-indexed) is a prefix of the pool. -/
theorem prefixCuts_getElem?_prefix {α : Type _} {pool : List α} {acc : Nat}
    {d : List Nat} {t : Nat} {v : List α}
    (h : (prefixCuts pool acc d)[t]? = some v) : v <+: pool := by
  obtain ⟨k, rfl⟩ := prefixCuts_mem_take (Trace.mem_of_read h)
  exact List.take_prefix k pool

/-- Prefix-cut reads ascend along ticks (option-indexed): the earlier
view is a prefix of the later. -/
theorem prefixCuts_getElem?_mono {α : Type _} {pool : List α} {acc : Nat}
    {d : List Nat} {t t' : Nat} (h : t ≤ t') {v v' : List α}
    (hv : (prefixCuts pool acc d)[t]? = some v)
    (hv' : (prefixCuts pool acc d)[t']? = some v') : v <+: v' := by
  obtain ⟨ht, rfl⟩ := List.getElem?_eq_some_iff.mp hv
  obtain ⟨ht', rfl⟩ := List.getElem?_eq_some_iff.mp hv'
  exact prefixCuts_getElem_prefix h ht'

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
@[reducible] def ValueOrder.multiset (α : Type _) : ValueOrder (Multiset α) where
  le a b := a ≤ b
  le_refl _ := _root_.le_refl _
  le_trans h₁ h₂ := _root_.le_trans h₁ h₂

/-- `(s + {x}) + {y} = (s + {y}) + {x}` (the singleton-append
commutativity the entry pool pays). -/
theorem add_singleton_comm {α : Type _} (s : Multiset α) (x y : α) :
    s + {x} + {y} = s + {y} + {x} := by
  rw [Multiset.add_assoc, Multiset.add_assoc,
    Multiset.add_comm ({x} : Multiset α) {y}]

instance {α : Type _} :
    RightCommutative (fun (s : Multiset α) (e : α) => s + {e}) :=
  ⟨fun s x y => add_singleton_comm s x y⟩

/-- Accumulating a batch element-wise is accumulating the batch. -/
theorem foldl_add_singleton {α : Type _} (s b : Multiset α) :
    @Multiset.foldl _ _ (fun s e => s + {e})
      ⟨fun s x y => add_singleton_comm s x y⟩ s b = s + b := by
  induction b using Multiset.induction_on generalizing s with
  | empty => rw [Multiset.foldl_zero, Multiset.add_zero]
  | cons x m ih =>
    rw [Multiset.foldl_cons, ih, ← Multiset.singleton_add,
      ← Multiset.add_assoc]

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

/-- Lift a `getElem` along a prefix (the stage-transport eliminator):
the deeper trace agrees with the shallower one below its length, and
the bound lifts with it. -/
theorem prefix_getElem_lift {α : Type _} {l l' : List α}
    (h : l <+: l') {t : Nat} (ht : t < l.length) :
    l'[t]'(Nat.lt_of_lt_of_le ht h.length_le) = l[t]'ht :=
  (List.IsPrefix.getElem h ht).symm

/-- **Tick-axis invariant transfer**: a binary trace property
preserved by appending ONE tick (in lockstep on two legs) transfers
along any prefix extension. The only way a ticked collection grows is
by whole ticks — so a one-tick proof is the whole proof. (The
stage-axis bridge for invariants over tick-typed `fix` wires, where a
Kleene step only appends ticks — `stages_mono` at `m ≤ m + 1`.) -/
theorem prefix_ext_invariant {β σ : Type _}
    (P : List β → List σ → Prop)
    {out out' : List β} {upd upd' : List σ}
    (hout : out <+: out') (hupd : upd <+: upd')
    (hlen : out.length = upd.length)
    (hlen' : out'.length = upd'.length)
    (htick : ∀ (n : Nat), n < out'.length → out.length ≤ n →
      P (out'.take n) (upd'.take n) →
      P (out'.take (n + 1)) (upd'.take (n + 1)))
    (ih : P out upd) : P out' upd' := by
  have hle : out.length ≤ out'.length := hout.length_le
  have key : ∀ (k n : Nat), n = out.length + k → n ≤ out'.length →
      P (out'.take n) (upd'.take n) := by
    intro k
    induction k with
    | zero =>
      intro n hn hnle
      subst hn
      rw [Nat.add_zero]
      rw [List.prefix_iff_eq_take.mp hout |>.symm]
      rw [hlen, List.prefix_iff_eq_take.mp hupd |>.symm]
      exact ih
    | succ k ihk =>
      intro n hn hnle
      subst hn
      rw [show out.length + (k + 1) = (out.length + k) + 1 by omega]
      exact htick (out.length + k) (by omega) (by omega)
        (ihk _ rfl (by omega))
  have hfin := key (out'.length - out.length) out'.length
    (by omega) (Nat.le_refl _)
  rw [List.take_length] at hfin
  rw [hlen'] at hfin
  rw [List.take_length] at hfin
  exact hfin

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

/-- **Bounded chain invariant** (the `fix` `invariant` clause's
engine): an invariant with base at the seed and step below the fuel
holds at the fuel — the step may consume chain facts relative to the
fuel (the `m < n` bound feeds the generated `stages_mono` premises). -/
theorem iterate_invariant {τ : Type _} {F : τ → τ} {x : τ}
    {I : τ → Prop} :
    ∀ (n : Nat), I x →
      (∀ m, m < n → I (iterate F x m) → I (F (iterate F x m))) →
      I (iterate F x n)
  | 0, hx, _ => hx
  | n + 1, hx, hstep =>
    hstep n (Nat.lt_succ_self n)
      (iterate_invariant n hx fun m hm ih =>
        hstep m (Nat.lt_succ_of_lt hm) ih)

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
recorded in `Hydro/README.md`.) -/
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

/-- **The consumed pool** of a realized batch run: the sum of the
count-legal increments the cut decision realized against `pool`
(`use::batch`'s `nondet!` as a value — every batch-fed contract speaks
of its input through this; `cq` = the quorum stage that named it). -/
def cqConsumed {α : Type _} [DecidableEq α] (pool : Multiset α)
    (d : List (Multiset α)) : Multiset α :=
  (batchCuts pool 0 d).sum

/-- The consumed pool never exceeds the source. -/
theorem cqConsumed_le {α : Type _} [DecidableEq α] (pool : Multiset α)
    (d : List (Multiset α)) : cqConsumed pool d ≤ pool := by
  unfold cqConsumed
  have := batchCuts_sum_le (pool := pool) (d := d) (consumed := 0)
    (Multiset.zero_le _)
  rwa [Multiset.zero_add] at this

/-- A consumed element is a pool element (one batch of the cut, summed,
sits below the pool). -/
theorem mem_pool_of_mem_batch {α : Type _} [DecidableEq α] {pool : Multiset α}
    {d : List (Multiset α)} {n : Nat} {b : Multiset α}
    (hb : (batchCuts pool 0 d)[n]? = some b) {a : α} (ha : a ∈ b) : a ∈ pool := by
  have hle := batchCuts_sum_le (pool := pool) (d := d) (consumed := 0) (Multiset.zero_le _)
  rw [Multiset.zero_add] at hle
  exact Multiset.mem_of_le hle (mem_list_sum.mpr ⟨b, Trace.mem_of_read hb, ha⟩)

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

/-- Two reads of a multiset trace at distinct ticks sit together below the
trace's sum (two consumed batches are two disjoint parts of the pool). -/
theorem Trace.two_reads_le_sum {α : Type _} [DecidableEq α] {L : Trace (Multiset α)}
    {w u : Nat} (hwu : w < u) {a b : Multiset α}
    (ha : L[w]? = some a) (hb : L[u]? = some b) : a + b ≤ L.sum := by
  obtain ⟨hu, rfl⟩ := List.getElem?_eq_some_iff.mp hb
  have ha' : a ∈ L.take u := by
    rw [List.mem_iff_getElem?]
    exact ⟨w, by rw [List.getElem?_take_of_lt hwu]; exact ha⟩
  have hle : a ≤ (L.take u).sum := by
    have := sublist_sum_le (List.singleton_sublist.mpr ha')
    rwa [List.sum_singleton] at this
  rw [← List.sum_take_add_sum_drop L u, List.drop_eq_getElem_cons hu, List.sum_cons]
  exact add_le_add hle (Multiset.le_add_right _ _)




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

end Hydro
