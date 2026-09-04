import Hydro.Transfer
import Hydro.SchedCausal

/-!
# Hydro · the coupling corner (`CoupleSem`) — derived decisions at an ambient horizon

The D39 redesign of machine-run transfer (supersedes the δ-accumulating
square for end-to-end theorems; see `SCHED_AUDIT.md` F1 and FINDINGS
D37–D39). The human-level induction it internalizes:

> *if the machine run extends by a step, then wherever a denotational
> `nondet!` sits, the decision made so far extends to a matching one —
> no matter what the prefix was.*

Formally: every derive function (`batchDerive`, `snapDerive`,
`ordSelDerive`, `batchOrdDerive`) is horizon-monotone, so the decision
matching a machine prefix is *computed*, never postulated. This
interpretation bakes that in:

- the **horizon `T` is ambient** (an instance parameter, like
  `pacing`), so every wire couples its machine leg's view at `T`
  against its denotational leg;
- **decision-mediated ops derive their own decisions at `T`** — there
  is no decision environment `D`, no lens record, no legality atom:
  the coupling proof of each op is the realization theorem at its own
  derived decision (`corr_*` + `prefix_refl`), and knot fuel is `T+1`
  by construction (the fuel↔horizon matching);
- carriers are **corners**, not squares: a machine leg `sr`, a plain
  denotational leg `rr`, and a coupling `cpl` conditioned on a single
  residual `Prop` (`wf`) that only `fix` populates (an opaque loop body
  owes naturality of its legs, ascent of its Kleene chain, and its
  stage chain's `wf` — dischargeable per knot from the program's
  instance-generic bodies: naturality by projection-`rfl` transport,
  ascent by one `MonoRel` instantiation).

Streams/keyed/fold carriers couple at `T` only (their machine legs are
prefix-monotone histories, so earlier views couple by restriction);
tick-trace carriers couple at **every step `≤ T`** (tick entries are
not one growing list, and the freeze/flatten ops read them at
freeze-chosen earlier steps).
-/

namespace Hydro

/-! ## Carriers -/

/-- Stream corner: machine step-histories vs a denotational pool,
coupled at the ambient horizon (earlier views couple by
`StepHist.mono`). -/
structure CoStream (T : Nat) (n : Nat) (α : Type) [DecidableEq α]
    (ord : StrOrd) (ret : Retries) : Type where
  sr : Fin n → StepHist α
  rr : Fin n → PoolCarrier α ord ret
  wf : Prop
  cpl : wf → ∀ i, ListLe ord ret ((sr i).view T) (rr i)

structure CoKeyed (T : Nat) (p c : Nat) (α : Type) [DecidableEq α]
    (ord : StrOrd) (ret : Retries) : Type where
  sr : Fin p → Fin c → StepHist α
  rr : Fin p → Fin c → PoolCarrier α ord ret
  wf : Prop
  cpl : wf → ∀ i j, ListLe ord ret ((sr i j).view T) (rr i j)

/-- Fold corner (mirrors `SqSing.cpl`'s existential package). -/
structure CoSing (T : Nat) (n : Nat) (α σ : Type) [DecidableEq α]
    (ord : StrOrd) (ret : Retries) (b : SingBound σ) : Type where
  sr : Fin n → SchedFold α σ
  rr : SingletonV n α σ ord b
  wf : Prop
  cpl : wf → ∃ (g : σ → α → σ) (init : σ)
    (ok : FoldOkP ord ret g) (pool : Fin n → PoolCarrier α ord ret),
    (∀ i, singReads b rr i = snapTrace ord ret g init ok (pool i)) ∧
    (∀ i, (sr i).read = fun l => l.foldl g init) ∧
    (∀ i, ListLe ord ret ((sr i).src.view T) (pool i))

/-- Ticked corner: coupled at every step `≤ T`. -/
structure CoTickSing (T : Nat) (n : Nat) (σ : Type) : Type where
  sr : Fin n → Nat → Trace σ
  rr : TickV n σ
  wf : Prop
  cpl : wf → ∀ k, k ≤ T → ∀ i, sr i k <+: rr i

/-- Tick-stream corner: coupled at every step `≤ T`. -/
structure CoTickStream (T : Nat) (n : Nat) (α : Type) [DecidableEq α]
    (ord : StrOrd) (ret : Retries) : Type where
  sr : Fin n → Nat → Trace (List α)
  rr : Fin n → Trace (PoolCarrier α ord ret)
  wf : Prop
  cpl : wf → ∀ k, k ≤ T → ∀ i, BatchTraceLe ord ret (sr i k) (rr i)

/-! ## One tick's bounded stream, coupled (D60)

Inside a tick body the corner carries BOTH representations of one
tick's content — the machine's concrete `List` and the denotation's
grade quotient — with the same `wf`-guarded coupling discipline as
every wire carrier. `CoupledBatch` is the per-tick relation (at
`ExactlyOnce` it is *equality*: a tick's batch is the consumed
increment, identical on both legs; the retry grades carry no per-tick
coupling, exactly as `BatchTraceLe`). Stream-typed in-tick operators
act legwise and transport the coupling (one lemma each — the proof the
quotient-vs-list split exists to force); plain-typed results (`count`,
`fold`, `first`) compute from the machine leg, and their agreement
with the denotation is the conditional co-rule (`wf → …`) their grade
constraint makes provable. -/

/-- The per-tick batch coupling: machine list vs denotation content. -/
@[reducible] def CoupledBatch {α : Type} [DecidableEq α] :
    (ord : StrOrd) → (ret : Retries) → List α →
      PoolCarrier α ord ret → Prop
  | .totalOrder, .exactlyOnce => fun s v => s = v
  | .noOrder, .exactlyOnce => fun s v => (↑s : Multiset α) = v
  | .totalOrder, .atLeastOnce => fun _ _ => True
  | .noOrder, .atLeastOnce => fun _ _ => True

/-- One tick's bounded stream at the corner. -/
structure CoBounded (α : Type) [DecidableEq α] (ord : StrOrd)
    (ret : Retries) : Type where
  sr : List α
  rr : PoolCarrier α ord ret
  wf : Prop
  cpl : wf → CoupledBatch ord ret sr rr

/-- This tick's singleton at the corner: both legs, coupled by
equality under `wf` (a tick singleton is a value; the two runs agree on
it exactly when their inputs were coupled). -/
structure CoBSing (σ : Type) : Type where
  sr : σ
  rr : σ
  wf : Prop
  cpl : wf → sr = rr

namespace CoBSing
variable {σ τ : Type}
def pure (v : σ) : CoBSing σ := ⟨v, v, True, fun _ => rfl⟩
def map (s : CoBSing σ) (f : σ → τ) : CoBSing τ :=
  ⟨f s.sr, f s.rr, s.wf, fun h => congrArg f (s.cpl h)⟩
def zip (a : CoBSing σ) (b : CoBSing τ) : CoBSing (σ × τ) :=
  ⟨(a.sr, b.sr), (a.rr, b.rr), a.wf ∧ b.wf,
   fun h => by rw [a.cpl h.1, b.cpl h.2]⟩
def unwrapOr (o : CoBSing (Option σ)) (s : CoBSing σ) : CoBSing σ :=
  ⟨o.sr.getD s.sr, o.rr.getD s.rr, o.wf ∧ s.wf,
   fun h => by rw [o.cpl h.1, s.cpl h.2]⟩
end CoBSing

namespace CoupledBatch

variable {α β : Type} [DecidableEq α] [DecidableEq β]

theorem map {ord : StrOrd} {s : List α} {v : PoolCarrier α ord .exactlyOnce}
    (h : CoupledBatch ord .exactlyOnce s v) (f : α → β) :
    CoupledBatch ord .exactlyOnce (s.map f) (mapPool (ord := ord) f v) := by
  cases ord
  · exact congrArg (List.map f) h
  · show (↑(s.map f) : Multiset β) = Multiset.map f v
    rw [← h, Multiset.map_coe]

theorem filterMap {ord : StrOrd} {s : List α}
    {v : PoolCarrier α ord .exactlyOnce}
    (h : CoupledBatch ord .exactlyOnce s v) (f : α → Option β) :
    CoupledBatch ord .exactlyOnce (s.filterMap f)
      (filterMapPool (ord := ord) f v) := by
  cases ord
  · exact congrArg (List.filterMap f) h
  · show (↑(s.filterMap f) : Multiset β) = Multiset.filterMap f v
    rw [← h, Multiset.filterMap_coe]

theorem chain {ord : StrOrd} {s t : List α}
    {v w : PoolCarrier α ord .exactlyOnce}
    (hs : CoupledBatch ord .exactlyOnce s v)
    (ht : CoupledBatch ord .exactlyOnce t w) :
    CoupledBatch ord .exactlyOnce (s ++ t) (poolChain (ord := ord) v w) := by
  cases ord
  · show s ++ t = v ++ w
    rw [hs, ht]
  · show (↑(s ++ t) : Multiset α) = v + w
    rw [← hs, ← ht, Multiset.coe_add]

theorem weakenOrder {ord : StrOrd} {s : List α}
    {v : PoolCarrier α ord .exactlyOnce}
    (h : CoupledBatch ord .exactlyOnce s v) :
    CoupledBatch .noOrder .exactlyOnce s (poolWeakenOrder (ord := ord) v) := by
  cases ord
  · show (↑s : Multiset α) = (↑v : Multiset α)
    rw [h]
  · exact h

theorem filter {ord : StrOrd} {s : List α}
    {v : PoolCarrier α ord .exactlyOnce}
    (h : CoupledBatch ord .exactlyOnce s v) (p : α → Bool) :
    CoupledBatch ord .exactlyOnce (s.filter p) (poolFilter (ord := ord) p v) := by
  cases ord
  · exact congrArg (List.filter p) h
  · show (↑(s.filter p) : Multiset α) = Multiset.filter (fun x => p x = true) v
    rw [← h, Multiset.filter_coe]
    simp only [Bool.decide_eq_true]

/-- The weakened content of a coupled batch is the batch's multiset. -/
theorem coe_eq_weaken {ord : StrOrd} {s : List α}
    {v : PoolCarrier α ord .exactlyOnce}
    (h : CoupledBatch ord .exactlyOnce s v) :
    (↑s : Multiset α) = poolWeakenOrder (ord := ord) v := by
  cases ord
  · show (↑s : Multiset α) = (↑v : Multiset α)
    rw [h]
  · exact h

theorem keyedFold {K V A : Type} [DecidableEq K] [DecidableEq V] [DecidableEq A]
    {ord : StrOrd} (g : A → V → A) (init : A) (ok : FoldOkP ord .exactlyOnce g)
    {s : List (K × V)} {v : PoolCarrier (K × V) ord .exactlyOnce}
    (h : CoupledBatch ord .exactlyOnce s v) :
    CoupledBatch .noOrder .exactlyOnce (keyedFoldList g init s)
      (poolKeyedFold (ord := ord) g init ok v) := by
  cases ord
  · show (↑(keyedFoldList g init s) : Multiset (K × A)) = ↑(keyedFoldList g init v)
    rw [h]
  · show (↑(keyedFoldList g init s) : Multiset (K × A)) = keyedFoldMultiset g init ok v
    rw [← h, coe_keyedFoldList g init ok s]

theorem join {K V W : Type} [DecidableEq K] [DecidableEq V] [DecidableEq W]
    {ord ord' : StrOrd} {s : List (K × V)} {v : PoolCarrier (K × V) ord .exactlyOnce}
    {t : List (K × W)} {w : PoolCarrier (K × W) ord' .exactlyOnce}
    (hs : CoupledBatch ord .exactlyOnce s v) (ht : CoupledBatch ord' .exactlyOnce t w) :
    CoupledBatch .noOrder .exactlyOnce (listJoin s t)
      (poolJoin (ord := ord) (ord' := ord') v w) := by
  show (↑(listJoin s t) : Multiset (K × (V × W))) = multisetJoin _ _
  rw [coe_listJoin, coe_eq_weaken hs, coe_eq_weaken ht]

theorem antiJoin {K V : Type} [DecidableEq K] [DecidableEq V] {ord ord' : StrOrd}
    {s : List (K × V)} {v : PoolCarrier (K × V) ord .exactlyOnce}
    {t : List K} {w : PoolCarrier K ord' .exactlyOnce}
    (hs : CoupledBatch ord .exactlyOnce s v) (ht : CoupledBatch ord' .exactlyOnce t w) :
    CoupledBatch ord .exactlyOnce (s.filter (fun e => !(decide (e.1 ∈ t))))
      (poolAntiJoin (ord := ord) (ord' := ord') v w) := by
  unfold poolAntiJoin
  rw [← coe_eq_weaken ht]
  have hp : (fun e : K × V => !(decide (e.1 ∈ (↑t : Multiset K))))
      = (fun e => !(decide (e.1 ∈ t))) := by
    funext e; simp only [Multiset.mem_coe]
  rw [hp]
  exact CoupledBatch.filter hs _

theorem filterNotIn {ord ord' : StrOrd}
    {s : List α} {v : PoolCarrier α ord .exactlyOnce}
    {t : List α} {w : PoolCarrier α ord' .exactlyOnce}
    (hs : CoupledBatch ord .exactlyOnce s v) (ht : CoupledBatch ord' .exactlyOnce t w) :
    CoupledBatch ord .exactlyOnce (s.filter (fun x => !(decide (x ∈ t))))
      (poolFilterNotIn (ord := ord) (ord' := ord') v w) := by
  unfold poolFilterNotIn
  rw [← coe_eq_weaken ht]
  have hp : (fun x : α => !(decide (x ∈ (↑t : Multiset α))))
      = (fun x => !(decide (x ∈ t))) := by
    funext x; simp only [Multiset.mem_coe]
  rw [hp]
  exact CoupledBatch.filter hs _

theorem max [LinearOrder α] {ord : StrOrd} {s : List α}
    {v : PoolCarrier α ord .exactlyOnce} (h : CoupledBatch ord .exactlyOnce s v) :
    s.foldl maxStep none = poolMax (ord := ord) v := by
  cases ord
  · exact congrArg (List.foldl maxStep none) h
  · show s.foldl maxStep none = Multiset.foldl maxStep none v
    rw [← h]
    rfl

theorem filterIf {ord : StrOrd} {ret : Retries} {s : List α}
    {v : PoolCarrier α ord ret} (h : CoupledBatch ord ret s v) (flag : Bool) :
    CoupledBatch ord ret (if flag then s else []) (poolFilterIf v flag) := by
  unfold poolFilterIf
  cases flag
  · simp only [Bool.false_eq_true, ↓reduceIte]
    cases ord <;> cases ret
    · rfl
    · trivial
    · exact Multiset.coe_nil
    · trivial
  · simpa using h

theorem flatMapUnordered {ord : StrOrd} {s : List α}
    {v : PoolCarrier α ord .exactlyOnce}
    (h : CoupledBatch ord .exactlyOnce s v)
    (f : α → List β) (g : α → Multiset β)
    (hfg : ∀ a ∈ s, (↑(f a) : Multiset β) = g a) :
    CoupledBatch .noOrder .exactlyOnce (s.flatMap f)
      (poolFlatMapUnordered (ord := ord) v g) := by
  show (↑(s.flatMap f) : Multiset β) = _
  rw [← Multiset.coe_bind]
  cases ord
  · show _ = Multiset.bind (↑v) g
    rw [← h]
    exact Multiset.bind_congr (fun a ha => hfg a (Multiset.mem_coe.mp ha))
  · show _ = Multiset.bind v g
    rw [← h]
    exact Multiset.bind_congr (fun a ha => hfg a (Multiset.mem_coe.mp ha))

theorem count {ord : StrOrd} {s : List α} {v : PoolCarrier α ord .exactlyOnce}
    (h : CoupledBatch ord .exactlyOnce s v) :
    s.length = poolCount (ord := ord) v := by
  cases ord
  · exact congrArg List.length h
  · show s.length = Multiset.card v
    rw [← h, Multiset.coe_card]

theorem fold {ord : StrOrd} {ret : Retries} {σ : Type} (g : σ → α → σ)
    (init : σ) (ok : FoldOk ord ret g) {s : List α}
    {v : PoolCarrier α ord ret} (h : CoupledBatch ord ret s v)
    (hret : ret = .exactlyOnce) :
    s.foldl g init = PoolFold ord ret g init ok v := by
  subst hret
  cases ord
  · exact congrArg (List.foldl g init) h
  · show s.foldl g init = @Multiset.foldl α σ g ⟨fun s x y => ok s x y⟩ init v
    rw [← h]
    rfl

end CoupledBatch

/-! ## Knot combinators (stream) -/

namespace CoStream

variable {T : Nat} {n : Nat} {α : Type} [DecidableEq α]
  {ord : StrOrd} {ret : Retries}

/-- Coupling at an earlier horizon, by machine-history restriction. -/
theorem cpl_le (C : CoStream T n α ord ret) (hwf : C.wf) {k : Nat}
    (hk : k ≤ T) (i : Fin n) :
    ListLe ord ret ((C.sr i).view k) (C.rr i) :=
  ListLe.of_prefix ((C.sr i).mono_le hk) (C.cpl hwf i)

/-- Machine probe. -/
def schedEmbed (x : Fin n → StepHist α) : CoStream T n α ord ret where
  sr := x
  rr := fun _i => PoolBot ord ret
  wf := False
  cpl := fun hwf => absurd hwf (by simp)

/-- External input: a machine wire coupled to a denotational pool at
every horizon (the program-boundary hypothesis). -/
def input (h : Fin n → StepHist α) (v : Fin n → PoolCarrier α ord ret)
    (hc : ∀ t i, ListLe ord ret ((h i).view t) (v i)) :
    CoStream T n α ord ret where
  sr := h
  rr := v
  wf := True
  cpl := fun _ i => hc T i

/-- The two-leg probe (a machine wire *and* a reader, no coupling):
the knot's reader iteration reads its sites' machine context from the
knot's own diagonal through this probe, so every occurrence of a site
(top-level and in-knot) derives the *same* decision. -/
def probe2 (x : Fin n → StepHist α) (v : Fin n → PoolCarrier α ord ret) :
    CoStream T n α ord ret where
  sr := x
  rr := v
  wf := False
  cpl := fun hwf => absurd hwf (by simp)

/-- Lower the couple horizon (machine views are prefix-monotone, so a
coupling at `T` restricts to any `T' ≤ T`). -/
def lower (C : CoStream T n α ord ret) {T' : Nat} (h : T' ≤ T) :
    CoStream T' n α ord ret where
  sr := C.sr
  rr := C.rr
  wf := C.wf
  cpl := fun hwf i => C.cpl_le hwf h i

variable (body : CoStream T n α ord ret → CoStream T n α ord ret)

/-- The machine-side body. -/
def sbody (x : Fin n → StepHist α) : Fin n → StepHist α :=
  (body (schedEmbed x)).sr

/-- The knot's machine leg: the frozen diagonal of the machine body's
shift-iterates (the `SchedSem` fix semantics, verbatim). -/
def fixSr : Fin n → StepHist α :=
  famHist (fun t i =>
    ((iterate (fun x j => ((sbody body x) j).shift)
      (fun _ => StepHist.bot) (t + 1)) i).view t)

/-- The reader-side body at the knot's own machine diagonal: sites
inside the loop derive their decisions from the same machine wire the
closed knot presents to the rest of the program. -/
def vbody (v : Fin n → PoolCarrier α ord ret) :
    Fin n → PoolCarrier α ord ret :=
  (body (probe2 (fixSr body) v)).rr

end CoStream

/-! ## Knot combinators (tick) -/

/-- Lower the couple horizon (the fold-source coupling restricts along
machine-view monotonicity). -/
def CoSing.lower {T n : Nat} {α σ : Type} [DecidableEq α]
    {ord : StrOrd} {ret : Retries} {b : SingBound σ}
    (C : CoSing T n α σ ord ret b) {T' : Nat} (h : T' ≤ T) :
    CoSing T' n α σ ord ret b where
  sr := C.sr
  rr := C.rr
  wf := C.wf
  cpl := fun hwf => by
    obtain ⟨g, init, ok, pool, hr, hf, hsrc⟩ := C.cpl hwf
    exact ⟨g, init, ok, pool, hr, hf, fun i =>
      ListLe.of_prefix ((C.sr i).src.mono_le h) (hsrc i)⟩

namespace CoTickSing

variable {T : Nat} {n : Nat} {σ : Type}

def schedEmbed (x : Fin n → Nat → Trace σ) :
    CoTickSing T n σ where
  sr := x
  rr := fun _i => ([] : Trace σ)
  wf := False
  cpl := fun hwf => absurd hwf (by simp)

/-- External input (tick-singleton boundary). -/
def input (s : Fin n → Nat → Trace σ) (v : TickV n σ)
    (hc : ∀ t i, s i t <+: v i) : CoTickSing T n σ where
  sr := s
  rr := v
  wf := True
  cpl := fun _ k _hk i => hc k i

/-- Two-leg probe (tick). -/
def probe2 (x : Fin n → Nat → Trace σ) (v : TickV n σ) :
    CoTickSing T n σ where
  sr := x
  rr := v
  wf := False
  cpl := fun hwf => absurd hwf (by simp)

/-- Lower the couple horizon (tick couplings quantify all steps `≤ T`,
so restriction is inclusion). -/
def lower (C : CoTickSing T n σ) {T' : Nat} (h : T' ≤ T) :
    CoTickSing T' n σ where
  sr := C.sr
  rr := C.rr
  wf := C.wf
  cpl := fun hwf k hk i => C.cpl hwf k (hk.trans h) i

variable (body : CoTickSing T n σ →
  CoTickSing T n σ)

def sbody (x : Fin n → Nat → Trace σ) : Fin n → Nat → Trace σ :=
  (body (schedEmbed x)).sr

/-- The machine-side body with the tick shift. -/
def sshift (x : Fin n → Nat → Trace σ) : Fin n → Nat → Trace σ :=
  fun j u => match u with
    | 0 => ([] : Trace σ)
    | u + 1 => (sbody body x) j u

/-- The knot's machine leg: the raw tick diagonal. -/
def fixSr : Fin n → Nat → Trace σ :=
  fun i t => (iterate (sshift body) (fun _ _ => ([] : Trace σ)) (t + 1)) i t

def vbody (v : TickV n σ) : TickV n σ :=
  (body (probe2 (fixSr body) v)).rr

end CoTickSing

/-! ## The knot coupling, generically

The one place a residual obligation lives. For an opaque loop body the
knot must know (its `wf` components):

- **`hcaus`** — the machine body is causal (views at `h` from views
  `≤ h`): makes the machine diagonal a genuine fixpoint of the body
  (`fix_view_succ`) and the freeze transparent;
- **`hchain`** — the reader body ascends its Kleene chain;
- **`hcplj`** — the *graded coupling*: at every horizon `j ≤ T`, if the
  probe legs couple below `j` then the body's legs couple at `j`. The
  program discharges this by re-instantiating its `∀ H'`-generic body
  at the `j`-lowered interpretation (`CoupleSem … j Td`) — the payoff
  of instance-generic knot bodies: the induction over machine prefixes
  is carried by the interpretation, not by a tactic walking the body.

`co_fix_cpl` then couples the knot by induction on the horizon: a new
machine step appears one body application above the diagonal
(`fix_view_succ`), and the graded coupling extends the derived
decisions to match — no stage chain, no telescope, no per-site
reasoning. -/

section KnotCpl

variable {T : Nat} {n : Nat} {α : Type} [DecidableEq α]
  {ord : StrOrd} {ret : Retries}
  (body : CoStream T n α ord ret → CoStream T n α ord ret)

/-- Re-monotonization is a no-op on step-ascending families. -/
private theorem co_famFreeze_eq_of_ascending {m : Nat} {β : Type}
    [DecidableEq β] (f : Nat → Fin m → List β) :
    ∀ t, (∀ t', t' < t → ∀ i, f t' i <+: f (t' + 1) i) →
      famFreeze f t = f t
  | 0, _ => rfl
  | t + 1, hasc => by
    show (if (List.finRange m).all
        (fun i => ((famFreeze f t) i).isPrefixOf (f (t + 1) i))
      then f (t + 1) else famFreeze f t) = f (t + 1)
    rw [co_famFreeze_eq_of_ascending f t
      (fun t' ht' i => hasc t' (Nat.lt_succ_of_lt ht') i)]
    rw [if_pos]
    exact List.all_eq_true.mpr (fun i _ =>
      List.isPrefixOf_iff_prefix.mpr (hasc t (Nat.lt_succ_self t) i))

variable
  (hcaus : ∀ (h : Nat) (x y : Fin n → StepHist α), SAgree h x y →
    SAgree h (CoStream.sbody body x) (CoStream.sbody body y))

include hcaus

/-- The raw diagonal ascends step by step (causality). -/
theorem co_diag_ascent : ∀ (t : Nat) (i : Fin n),
    ((iterate (fun x j => ((CoStream.sbody body x) j).shift)
      (fun _ => StepHist.bot) (t + 1)) i).view t
    <+: ((iterate (fun x j => ((CoStream.sbody body x) j).shift)
      (fun _ => StepHist.bot) (t + 2)) i).view (t + 1) := by
  intro t i
  calc ((iterate (fun x j => ((CoStream.sbody body x) j).shift)
        (fun _ => StepHist.bot) (t + 1)) i).view t
      = ((iterate (fun x j => ((CoStream.sbody body x) j).shift)
        (fun _ => StepHist.bot) (t + 2)) i).view t :=
      stab_iterate_shift (h := t)
        (fun h' _hh' x y hxy => hcaus h' x y hxy)
        t (t + 1) (t + 2) (Nat.le_refl _)
        (Nat.lt_succ_self _) (by omega) i
    _ <+: _ := ((iterate (fun x j => ((CoStream.sbody body x) j).shift)
        (fun _ => StepHist.bot) (t + 2)) i).mono t

/-- The frozen diagonal *is* the raw diagonal (causality ⇒ ascent ⇒
transparency). -/
theorem co_fix_view (t : Nat) (i : Fin n) :
    (CoStream.fixSr body i).view t
      = ((iterate (fun x j => ((CoStream.sbody body x) j).shift)
        (fun _ => StepHist.bot) (t + 1)) i).view t := by
  show famFreeze (fun t' j =>
      ((iterate (fun x j' => ((CoStream.sbody body x) j').shift)
        (fun _ => StepHist.bot) (t' + 1)) j).view t') t i = _
  rw [co_famFreeze_eq_of_ascending _ t
    (fun t' _ht' j => co_diag_ascent body hcaus t' j)]

/-- **The machine fixpoint equation**: one more body application on
the diagonal shows the diagonal's next view. -/
theorem co_fix_view_succ (t : Nat) (i : Fin n) :
    (CoStream.fixSr body i).view (t + 1)
      = ((CoStream.sbody body (CoStream.fixSr body)) i).view t := by
  rw [co_fix_view body hcaus (t + 1) i]
  show ((CoStream.sbody body
      (iterate (fun x j => ((CoStream.sbody body x) j).shift)
        (fun _ => StepHist.bot) (t + 1)) i).shift).view (t + 1) = _
  show ((CoStream.sbody body
      (iterate (fun x j => ((CoStream.sbody body x) j).shift)
        (fun _ => StepHist.bot) (t + 1))) i).view t = _
  refine hcaus t _ _ (fun j t' ht' => ?_) i t (Nat.le_refl _)
  rw [co_fix_view body hcaus t' j]
  exact stab_iterate_shift (h := t')
    (fun h' _hh' x y hxy => hcaus h' x y hxy)
    t' (t + 1) (t' + 1) (Nat.le_refl _)
    (by omega) (Nat.lt_succ_self _) j

/-- **The knot coupling**, by induction on the horizon. -/
theorem co_fix_cpl
    (hcplj : ∀ j, j ≤ T → ∀ (x : Fin n → StepHist α)
      (v : Fin n → PoolCarrier α ord ret),
      (∀ t, t ≤ j → ∀ i, ListLe ord ret ((x i).view t) (v i)) →
      ∀ i, ListLe ord ret ((CoStream.sbody body x i).view j)
        ((body (CoStream.probe2 x v)).rr i)) :
    ∀ j, j ≤ T → ∀ i, ListLe ord ret ((CoStream.fixSr body i).view j)
      (iterate (CoStream.vbody body)
        (fun _ => PoolBot ord ret) (j + 1) i)
  | 0, _hj, i => by
    rw [co_fix_view body hcaus 0 i]
    exact ListLe.nil _ _ _
  | j + 1, hj, i => by
    rw [co_fix_view_succ body hcaus j i]
    refine hcplj j (by omega) _ _ (fun t ht i' => ?_) i
    refine ListLe.of_prefix ((CoStream.fixSr body i').mono_le ht) ?_
    exact co_fix_cpl hcplj j (by omega) i'

end KnotCpl

/-! ## The knot coupling (tick) -/

section TickKnotCpl

variable {T : Nat} {n : Nat} {σ : Type}
  (body : CoTickSing T n σ → CoTickSing T n σ)
  (hcaus : ∀ (h : Nat) (x y : Fin n → Nat → Trace σ), TAgree h x y →
    TAgree h (CoTickSing.sbody body x) (CoTickSing.sbody body y))

include hcaus

/-- The tick fixpoint equation (raw diagonal, no freeze). -/
theorem co_tick_fix_succ (t : Nat) (i : Fin n) :
    CoTickSing.fixSr body i (t + 1)
      = CoTickSing.sbody body (CoTickSing.fixSr body) i t := by
  show (iterate (CoTickSing.sshift body)
      (fun _ _ => ([] : Trace σ)) (t + 2)) i (t + 1) = _
  show CoTickSing.sbody body
      (iterate (CoTickSing.sshift body)
        (fun _ _ => ([] : Trace σ)) (t + 1)) i t = _
  refine hcaus t _ _ (fun j t' ht' => ?_) i t (Nat.le_refl _)
  show (iterate (CoTickSing.sshift body)
      (fun _ _ => ([] : Trace σ)) (t + 1)) j t' = CoTickSing.fixSr body j t'
  exact stab_iterate_tick (h := t')
    (fun h' _hh' x y hxy => hcaus h' x y hxy)
    t' (t + 1) (t' + 1) (Nat.le_refl _)
    (by omega) (Nat.lt_succ_self _) j

/-- The tick knot coupling (the ascent hypothesis chains earlier
horizons' couplings up to the current iterate — tick entries are not
views of one history, so the chain is explicit). -/
theorem co_tick_fix_cpl
    (hchain : ∀ m i,
      (iterate (CoTickSing.vbody body)
        (fun _ => ([] : Trace σ)) m i)
      <+: (iterate (CoTickSing.vbody body)
        (fun _ => ([] : Trace σ)) (m + 1) i))
    (hcplj : ∀ j, j ≤ T → ∀ (x : Fin n → Nat → Trace σ)
      (v : TickV n σ),
      (∀ t, t ≤ j → ∀ i, x i t <+: v i) →
      ∀ i, CoTickSing.sbody body x i j
        <+: (body (CoTickSing.probe2 x v)).rr i) :
    ∀ j, j ≤ T → ∀ i, CoTickSing.fixSr body i j
      <+: (iterate (CoTickSing.vbody body)
        (fun _ => ([] : Trace σ)) (j + 1) i)
  | 0, _hj, i => by
    show (iterate (CoTickSing.sshift body)
      (fun _ _ => ([] : Trace σ)) 1) i 0 <+: _
    exact List.nil_prefix
  | j + 1, hj, i => by
    rw [co_tick_fix_succ body hcaus j i]
    refine hcplj j (by omega) _ _ (fun t ht i' => ?_) i
    refine (co_tick_fix_cpl hchain hcplj t (by omega) i').trans ?_
    refine trace_chain_glue (f := fun m =>
      iterate (CoTickSing.vbody body)
        (fun _ => ([] : Trace σ)) m i')
      (a := t + 1) (b := j + 1) (by omega) ?_
    intro m _hm _hmb
    exact hchain m i'

end TickKnotCpl

/-! ## The interpretation

Machine legs are the `SchedSem` semantics verbatim; denotational legs
are the `Values` ops at self-derived decisions; `wf` threads
conjunctively and only `fix` populates it. -/

/-- Delivery coupling: a cursor-delivered wire stays below the pool
its source is below (delivered views are takes of earlier source
views). -/
theorem deliver_view_cpl {α : Type} [DecidableEq α] {ord : StrOrd}
    {ret : Retries} (x : StepHist α) (cur : Nat → Nat)
    {v : PoolCarrier α ord ret} :
    ∀ T, ListLe ord ret (x.view T) v →
      ListLe ord ret ((x.deliver cur).view T) v
  | 0, _ => ListLe.nil _ _ _
  | u + 1, hsrc =>
    ListLe.take _ (ListLe.of_prefix (x.mono_le (Nat.le_succ u)) hsrc)

/-! ## Tick shapes at the corner (D60)

A shaped tick input is a tuple of coupled wires; the body runs ONCE,
as a Couple-typed function over coupled bounded tuples. Each leg of the
output is the body on a *leg-embedded* input (`embedSR`/`embedRR`: the
other leg dummy, `wf := False`), so the leg projections of `tick_scan`
are definitional (`co_tick_scan_sr/rr`). The coupling is proven from
three `wf` conjuncts about the body, each discharged by the `co_simp`
walk over the body's in-tick operators:

* **W1** the `sr` legs of the body depend only on the inputs' `sr` legs
  (the body on `embedSR` agrees with the body on the coupled embedding);
* **W2** likewise for the `rr` legs;
* **W3** the body's outputs are `wf` when its inputs are coupled.

Plus the shape-level fact that every stream leaf is `ExactlyOnce` (the
retry grades carry no per-tick coupling, as for `BatchTraceLe`). -/

namespace CoTick
variable {L : Type} {mem : L → Nat} {Tc : Nat}

abbrev Tk (Tc : Nat) (mem : L → Nat) (ℓ : L) (τ : Type) : Type :=
  CoTickSing Tc (mem ℓ) τ
abbrev TS (Tc : Nat) (mem : L → Nat) (ℓ : L) (α : Type) [DecidableEq α]
    (ord : StrOrd) (ret : Retries) : Type := CoTickStream Tc (mem ℓ) α ord ret

/-- Every stream leaf is `ExactlyOnce`. -/
def allEO : TickShape → Bool
  | .sing _ => true
  | .stream _ _ _ ret => match ret with | .exactlyOnce => true | .atLeastOnce => false
  | .pair a b => allEO a && allEO b

/-! ### Wires: the legs, the well-formedness, the coupling -/

def srLegs {ℓ : L} : (sh : TickShape) → TickedOf (Tk Tc mem) (TS Tc mem) ℓ sh →
    TickedOf (SchedTick.Tk mem) (SchedTick.TS mem) ℓ sh
  | .sing _, x => x.sr
  | .stream _ _ _ _, x => x.sr
  | .pair a b, x => (srLegs a x.1, srLegs b x.2)

def rrLegs {ℓ : L} : (sh : TickShape) → TickedOf (Tk Tc mem) (TS Tc mem) ℓ sh →
    TickedOf (ValuesTick.Tk mem) (ValuesTick.TS mem) ℓ sh
  | .sing _, x => x.rr
  | .stream _ _ _ _, x => x.rr
  | .pair a b, x => (rrLegs a x.1, rrLegs b x.2)

def Wf {ℓ : L} : (sh : TickShape) → TickedOf (Tk Tc mem) (TS Tc mem) ℓ sh → Prop
  | .sing _, x => x.wf
  | .stream _ _ _ _, x => x.wf
  | .pair a b, x => Wf a x.1 ∧ Wf b x.2

theorem cpl {ℓ : L} : (sh : TickShape) → (x : TickedOf (Tk Tc mem) (TS Tc mem) ℓ sh) →
    Wf sh x → ∀ k, k ≤ Tc → CorrTick.Le sh (srLegs sh x) k (rrLegs sh x)
  | .sing _, x, h, k, hk => x.cpl h k hk
  | .stream _ _ _ _, x, h, k, hk => x.cpl h k hk
  | .pair a b, x, h, k, hk => ⟨cpl a x.1 h.1 k hk, cpl b x.2 h.2 k hk⟩

def mk {ℓ : L} : (sh : TickShape) →
    (m : TickedOf (SchedTick.Tk mem) (SchedTick.TS mem) ℓ sh) →
    (v : TickedOf (ValuesTick.Tk mem) (ValuesTick.TS mem) ℓ sh) →
    (wf : Prop) → (wf → ∀ k, k ≤ Tc → CorrTick.Le sh m k v) →
    TickedOf (Tk Tc mem) (TS Tc mem) ℓ sh
  | .sing _, m, v, wf, h => ⟨m, v, wf, h⟩
  | .stream _ _ _ _, m, v, wf, h => ⟨m, v, wf, h⟩
  | .pair a b, m, v, wf, h =>
    (mk a m.1 v.1 wf (fun hw k hk => (h hw k hk).1),
     mk b m.2 v.2 wf (fun hw k hk => (h hw k hk).2))

/-! ### Bounded tuples: projections, embeddings, coercion -/

def projSR : (sh : TickShape) → BoundedOf CoBSing CoBounded sh →
    BoundedOf SchedTick.BS SchedTick.BStr sh
  | .sing _, b => b.sr
  | .stream _ _ _ _, b => b.sr
  | .pair a c, b => (projSR a b.1, projSR c b.2)

def projRR : (sh : TickShape) → BoundedOf CoBSing CoBounded sh →
    BoundedOf ValuesTick.BS PoolCarrier sh
  | .sing _, b => b.rr
  | .stream _ _ _ _, b => b.rr
  | .pair a c, b => (projRR a b.1, projRR c b.2)

def WfB : (sh : TickShape) → BoundedOf CoBSing CoBounded sh → Prop
  | .sing _, b => b.wf
  | .stream _ _ _ _, b => b.wf
  | .pair a c, b => WfB a b.1 ∧ WfB c b.2

/-- The machine leg alone (the denotation leg dummy, uncoupled). -/
def embedSR : (sh : TickShape) → BoundedOf SchedTick.BS SchedTick.BStr sh →
    BoundedOf CoBSing CoBounded sh
  | .sing _, v => ⟨v, v, False, fun h => h.elim⟩
  | .stream _ _ ord ret, l => ⟨l, PoolBot ord ret, False, fun h => h.elim⟩
  | .pair a c, b => (embedSR a b.1, embedSR c b.2)

/-- The denotation leg alone (the machine leg dummy, uncoupled). -/
def embedRR : (sh : TickShape) → BoundedOf ValuesTick.BS PoolCarrier sh →
    BoundedOf CoBSing CoBounded sh
  | .sing _, v => ⟨v, v, False, fun h => h.elim⟩
  | .stream _ _ _ _, p => ⟨[], p, False, fun h => h.elim⟩
  | .pair a c, b => (embedRR a b.1, embedRR c b.2)

/-- Both legs, `wf` exactly when coupled. -/
def embed2 : (sh : TickShape) → BoundedOf SchedTick.BS SchedTick.BStr sh →
    BoundedOf ValuesTick.BS PoolCarrier sh → BoundedOf CoBSing CoBounded sh
  | .sing _, m, v => ⟨m, v, m = v, id⟩
  | .stream _ _ ord ret, m, v => ⟨m, v, CoupledBatch ord ret m v, id⟩
  | .pair a c, m, v => (embed2 a m.1 v.1, embed2 c m.2 v.2)

/-- The machine tuple as the denotation's. -/
def coerce : (sh : TickShape) → BoundedOf SchedTick.BS SchedTick.BStr sh →
    BoundedOf ValuesTick.BS PoolCarrier sh
  | .sing _, v => v
  | .stream _ _ ord ret, l => poolOfListR ord ret l
  | .pair a c, b => (coerce a b.1, coerce c b.2)

theorem allEO_stream {α : Type} [DecidableEq α] {ord : StrOrd} {ret : Retries}
    (h : allEO (.stream α inferInstance ord ret) = true) : ret = .exactlyOnce := by
  cases ret
  · rfl
  · exact absurd h (by simp [allEO])

theorem allEO_pair {a b : TickShape} (h : allEO (.pair a b) = true) :
    allEO a = true ∧ allEO b = true := by
  simpa [allEO, Bool.and_eq_true] using h

/-- A coupled, well-formed tuple: the coerced machine leg IS the
denotation leg. -/
theorem coerce_of_wf : (sh : TickShape) → allEO sh = true →
    (b : BoundedOf CoBSing CoBounded sh) → WfB sh b →
    coerce sh (projSR sh b) = projRR sh b
  | .sing _, _, b, h => b.cpl h
  | .stream _ _ ord ret, heo, b, h => by
    obtain rfl := allEO_stream heo
    have hc := b.cpl h
    cases ord
    · exact hc
    · exact hc
  | .pair a c, heo, b, h => by
    show (coerce a (projSR a b.1), coerce c (projSR c b.2)) = (projRR a b.1, projRR c b.2)
    rw [coerce_of_wf a (allEO_pair heo).1 b.1 h.1,
      coerce_of_wf c (allEO_pair heo).2 b.2 h.2]

theorem zip_map_both {α β γ δ : Type _} (f : α → γ) (g : β → δ)
    (l : List α) (r : List β) :
    Trace.zip (l.map f) (r.map g) = (Trace.zip l r).map (fun p => (f p.1, g p.2)) := by
  rw [zip_map_left', zip_map_right', List.map_map]
  rfl

/-- The leafwise wire coupling, as a prefix of coerced input tuples. -/
theorem slice_le {ℓ : L} : (sh : TickShape) → allEO sh = true →
    (m : TickedOf (SchedTick.Tk mem) (SchedTick.TS mem) ℓ sh) → (k : Nat) →
    (v : TickedOf (ValuesTick.Tk mem) (ValuesTick.TS mem) ℓ sh) →
    CorrTick.Le sh m k v → ∀ i,
    (SchedTick.slice sh m i k).map (coerce sh) <+: ValuesTick.slice sh v i
  | .sing _, _, m, k, v, h, i => by
    show (m i k).map (fun x => x) <+: v i
    rw [List.map_id']
    exact h i
  | .stream _ _ ord ret, heo, m, k, v, h, i => by
    obtain rfl := allEO_stream heo
    have hc := h i
    cases ord
    · show (m i k).map (fun x => x) <+: v i
      rw [List.map_id']
      exact hc
    · exact hc
  | .pair a c, heo, m, k, v, h, i => by
    show (Trace.zip (SchedTick.slice a m.1 i k) (SchedTick.slice c m.2 i k)).map
        (fun p => (coerce a p.1, coerce c p.2))
      <+: Trace.zip (ValuesTick.slice a v.1 i) (ValuesTick.slice c v.2 i)
    rw [← zip_map_both]
    exact zip_prefix (slice_le a (allEO_pair heo).1 m.1 k v.1 h.1 i)
      (slice_le c (allEO_pair heo).2 m.2 k v.2 h.2 i)

theorem map_coerce_sing (τ : Type) :
    ∀ (l : List (BoundedOf SchedTick.BS SchedTick.BStr (.sing τ))),
      l.map (coerce (.sing τ)) = l
  | [] => rfl
  | _ :: xs => congrArg (List.cons _) (map_coerce_sing τ xs)

theorem map_coerce_total (α : Type) (inst : DecidableEq α) :
    ∀ (l : List (BoundedOf SchedTick.BS SchedTick.BStr
        (.stream α inst .totalOrder .exactlyOnce))),
      l.map (coerce (.stream α inst .totalOrder .exactlyOnce)) = l
  | [] => rfl
  | _ :: xs => congrArg (List.cons _) (map_coerce_total α inst xs)

theorem map_fst_map_coerce (a c : TickShape) :
    ∀ (l : List (BoundedOf SchedTick.BS SchedTick.BStr (.pair a c))),
      (l.map (coerce (.pair a c))).map Prod.fst = (l.map Prod.fst).map (coerce a)
  | [] => rfl
  | _ :: xs => congrArg (List.cons _) (map_fst_map_coerce a c xs)

theorem map_snd_map_coerce (a c : TickShape) :
    ∀ (l : List (BoundedOf SchedTick.BS SchedTick.BStr (.pair a c))),
      (l.map (coerce (.pair a c))).map Prod.snd = (l.map Prod.snd).map (coerce c)
  | [] => rfl
  | _ :: xs => congrArg (List.cons _) (map_snd_map_coerce a c xs)

/-- Back from coerced output-tuple prefixes to the leafwise coupling. -/
theorem unslice_le {ℓ : L} : (sh : TickShape) → allEO sh = true →
    (F : Fin (mem ℓ) → Nat → Trace (BoundedOf SchedTick.BS SchedTick.BStr sh)) →
    (k : Nat) → (G : Fin (mem ℓ) → Trace (BoundedOf ValuesTick.BS PoolCarrier sh)) →
    (∀ i, (F i k).map (coerce sh) <+: G i) →
    CorrTick.Le sh (SchedTick.unslice sh F) k (ValuesTick.unslice sh G)
  | .sing τ, _, F, k, G, h => fun i => by
    have := h i
    rw [map_coerce_sing τ] at this
    exact this
  | .stream α inst ord ret, heo, F, k, G, h => fun i => by
    obtain rfl := allEO_stream heo
    have hc := h i
    cases ord
    · rw [map_coerce_total α inst] at hc
      exact hc
    · exact hc
  | .pair a c, heo, F, k, G, h => by
    refine ⟨unslice_le a (allEO_pair heo).1 _ k _ (fun i => ?_),
      unslice_le c (allEO_pair heo).2 _ k _ (fun i => ?_)⟩
    · have := List.IsPrefix.map Prod.fst (h i)
      rw [map_fst_map_coerce a c] at this
      exact this
    · have := List.IsPrefix.map Prod.snd (h i)
      rw [map_snd_map_coerce a c] at this
      exact this

/-- The machine's initial register, coerced, is the denotation's (a
stream register starts empty on both legs). -/
theorem coerce_seed : (sh : TickShape) → (s : SeedOf sh) →
    coerce sh (SchedTick.seed sh s) = ValuesTick.seed sh s
  | .sing _, _ => rfl
  | .stream _ _ ord ret, _ => by cases ord <;> cases ret <;> rfl
  | .pair a b, s => by
    show (coerce a (SchedTick.seed a s.1), coerce b (SchedTick.seed b s.2)) = _
    rw [coerce_seed a s.1, coerce_seed b s.2]
    rfl

/-! ### The body's legs (the register is a shape like the inputs) -/

section Body
variable {ℓ : L} (sts ins outs : TickShape)
  (g : Fin (mem ℓ) → BoundedOf CoBSing CoBounded sts → BoundedOf CoBSing CoBounded ins →
    BoundedOf CoBSing CoBounded sts × BoundedOf CoBSing CoBounded outs)

/-- The machine body: the body on the machine leg alone. -/
def gS (i : Fin (mem ℓ)) (s : BoundedOf SchedTick.BS SchedTick.BStr sts)
    (inp : BoundedOf SchedTick.BS SchedTick.BStr ins) :
    BoundedOf SchedTick.BS SchedTick.BStr sts × BoundedOf SchedTick.BS SchedTick.BStr outs :=
  let r := g i (embedSR sts s) (embedSR ins inp)
  (projSR sts r.1, projSR outs r.2)

/-- The denotation body: the body on the denotation leg alone. -/
def gR (i : Fin (mem ℓ)) (s : BoundedOf ValuesTick.BS PoolCarrier sts)
    (inp : BoundedOf ValuesTick.BS PoolCarrier ins) :
    BoundedOf ValuesTick.BS PoolCarrier sts × BoundedOf ValuesTick.BS PoolCarrier outs :=
  let r := g i (embedRR sts s) (embedRR ins inp)
  (projRR sts r.1, projRR outs r.2)

/-- The coupled run: the body on a machine register and tuple coupled
with their own coercions. -/
def gC (i : Fin (mem ℓ)) (s : BoundedOf SchedTick.BS SchedTick.BStr sts)
    (inp : BoundedOf SchedTick.BS SchedTick.BStr ins) :
    BoundedOf CoBSing CoBounded sts × BoundedOf CoBSing CoBounded outs :=
  g i (embed2 sts s (coerce sts s)) (embed2 ins inp (coerce ins inp))

/-- W1: the machine legs are leg-independent (componentwise: the
register and the emissions). -/
def W1 : Prop := ∀ i s inp,
  (gS sts ins outs g i s inp).1 = projSR sts (gC sts ins outs g i s inp).1
    ∧ (gS sts ins outs g i s inp).2 = projSR outs (gC sts ins outs g i s inp).2
/-- W2: the denotation legs are leg-independent. -/
def W2 : Prop := ∀ i s inp,
  (gR sts ins outs g i (coerce sts s) (coerce ins inp)).1 = projRR sts (gC sts ins outs g i s inp).1
    ∧ (gR sts ins outs g i (coerce sts s) (coerce ins inp)).2 = projRR outs (gC sts ins outs g i s inp).2
/-- W3: coupled inputs give well-formed outputs. -/
def W3 : Prop := ∀ i s inp,
  WfB sts (gC sts ins outs g i s inp).1 ∧ WfB outs (gC sts ins outs g i s inp).2

/-- The machine body, coerced, is the denotation body on the coerced
register and input (pointwise). -/
theorem body_comm (heoS : allEO sts = true) (heo : allEO outs = true)
    (h1 : W1 sts ins outs g) (h2 : W2 sts ins outs g)
    (h3 : W3 sts ins outs g) (i : Fin (mem ℓ)) (s : BoundedOf SchedTick.BS SchedTick.BStr sts)
    (inp : BoundedOf SchedTick.BS SchedTick.BStr ins) :
    gR sts ins outs g i (coerce sts s) (coerce ins inp)
      = (coerce sts (gS sts ins outs g i s inp).1, coerce outs (gS sts ins outs g i s inp).2) := by
  have hw := (h3 i s inp).1
  have hwb := (h3 i s inp).2
  have h1a := (h1 i s inp).1
  have h1b := (h1 i s inp).2
  have h2a := (h2 i s inp).1
  have h2b := (h2 i s inp).2
  refine Prod.ext ?_ ?_
  · show (gR sts ins outs g i (coerce sts s) (coerce ins inp)).1
      = coerce sts (gS sts ins outs g i s inp).1
    rw [h2a, h1a, coerce_of_wf sts heoS _ hw]
  · show (gR sts ins outs g i (coerce sts s) (coerce ins inp)).2
      = coerce outs (gS sts ins outs g i s inp).2
    rw [h2b, h1b, coerce_of_wf outs heo _ hwb]

theorem scan_comm (heoS : allEO sts = true) (heo : allEO outs = true)
    (h1 : W1 sts ins outs g) (h2 : W2 sts ins outs g)
    (h3 : W3 sts ins outs g) (i : Fin (mem ℓ)) :
    ∀ (l : Trace (BoundedOf SchedTick.BS SchedTick.BStr ins))
      (s : BoundedOf SchedTick.BS SchedTick.BStr sts),
      (scanAcrossTicksTrace (gS sts ins outs g i) s l).map (coerce outs)
        = scanAcrossTicksTrace (gR sts ins outs g i) (coerce sts s) (l.map (coerce ins))
  | [], _ => rfl
  | a :: l, s => by
    show coerce outs (gS sts ins outs g i s a).2 :: _
      = (gR sts ins outs g i (coerce sts s) (coerce ins a)).2
        :: scanAcrossTicksTrace _ (gR sts ins outs g i (coerce sts s) (coerce ins a)).1 _
    rw [body_comm sts ins outs g heoS heo h1 h2 h3 i s a]
    exact congrArg _ (scan_comm heoS heo h1 h2 h3 i l _)

end Body

/-- The corner's `wf` for a tick former: the inputs' well-formedness,
the shape-level `ExactlyOnce` facts (register, inputs, emissions), and
the body's three leg-independence / well-formedness conditions. -/
def scanWf {ℓ : L} (sts ins outs : TickShape)
    (x : TickedOf (Tk Tc mem) (TS Tc mem) ℓ ins)
    (g : Fin (mem ℓ) → BoundedOf CoBSing CoBounded sts → BoundedOf CoBSing CoBounded ins →
      BoundedOf CoBSing CoBounded sts × BoundedOf CoBSing CoBounded outs) : Prop :=
  Wf ins x ∧ allEO sts = true ∧ allEO ins = true ∧ allEO outs = true
    ∧ W1 sts ins outs g ∧ W2 sts ins outs g ∧ W3 sts ins outs g

/-- The coupling of the two legs' tick formers, under `scanWf`. -/
theorem scan_cpl (pacing : (ℓ : L) → Fin (mem ℓ) → Nat → Bool) {ℓ : L}
    (sts ins outs : TickShape) (x : TickedOf (Tk Tc mem) (TS Tc mem) ℓ ins)
    (g : Fin (mem ℓ) → BoundedOf CoBSing CoBounded sts → BoundedOf CoBSing CoBounded ins →
      BoundedOf CoBSing CoBounded sts × BoundedOf CoBSing CoBounded outs)
    (init : SeedOf sts) :
    scanWf sts ins outs x g → ∀ k, k ≤ Tc →
      CorrTick.Le outs
        ((SchedSem L mem pacing).tick_scan sts ins outs (srLegs ins x) (gS sts ins outs g) init) k
        ((Values L mem).tick_scan sts ins outs (rrLegs ins x) (gR sts ins outs g) init) :=
  fun ⟨hx, heoS, heoI, heoO, h1, h2, h3⟩ k hk =>
    unslice_le outs heoO
      (fun i k => scanAcrossTicksTrace (gS sts ins outs g i) (SchedTick.seed sts init)
        (SchedTick.slice ins (srLegs ins x) i k)) k
      (fun i => scanAcrossTicksTrace (gR sts ins outs g i) (ValuesTick.seed sts init)
        (ValuesTick.slice ins (rrLegs ins x) i))
      (fun i => by
        rw [scan_comm sts ins outs g heoS heoO h1 h2 h3 i, coerce_seed]
        exact scanAcrossTicksTrace_prefix _ (ValuesTick.seed sts init)
          (slice_le ins heoI _ k _ (cpl ins x hx k hk) i))

/-- The tick former at the corner: the machine's former on the `sr`
legs, the denotation's on the `rr` legs, coupled under `scanWf`. -/
def scan (pacing : (ℓ : L) → Fin (mem ℓ) → Nat → Bool) {ℓ : L}
    (sts ins outs : TickShape) (x : TickedOf (Tk Tc mem) (TS Tc mem) ℓ ins)
    (g : Fin (mem ℓ) → BoundedOf CoBSing CoBounded sts → BoundedOf CoBSing CoBounded ins →
      BoundedOf CoBSing CoBounded sts × BoundedOf CoBSing CoBounded outs)
    (init : SeedOf sts) :
    TickedOf (Tk Tc mem) (TS Tc mem) ℓ outs :=
  mk outs ((SchedSem L mem pacing).tick_scan sts ins outs (srLegs ins x) (gS sts ins outs g) init)
    ((Values L mem).tick_scan sts ins outs (rrLegs ins x) (gR sts ins outs g) init)
    (scanWf sts ins outs x g) (scan_cpl pacing sts ins outs x g init)

/-! ### The structural simp rules (the `co_simp` / `co_wf_simp` walk
over shaped tick formers: leg projections of `mk`, the leaf/pair
equations of every shape-recursive helper) -/

section Rules
variable {ℓ : L}

theorem srLegs_pair (a b : TickShape) (x : TickedOf (Tk Tc mem) (TS Tc mem) ℓ (.pair a b)) :
    srLegs (.pair a b) x = (srLegs a x.1, srLegs b x.2) := rfl
theorem srLegs_sing (τ : Type) (x : CoTickSing Tc (mem ℓ) τ) :
    srLegs (mem := mem) (ℓ := ℓ) (.sing τ) x = x.sr := rfl
theorem srLegs_stream (α : Type) (inst : DecidableEq α) (ord : StrOrd)
    (ret : Retries) (x : CoTickStream Tc (mem ℓ) α ord ret) :
    srLegs (mem := mem) (ℓ := ℓ) (.stream α inst ord ret) x = x.sr := rfl
theorem rrLegs_pair (a b : TickShape) (x : TickedOf (Tk Tc mem) (TS Tc mem) ℓ (.pair a b)) :
    rrLegs (.pair a b) x = (rrLegs a x.1, rrLegs b x.2) := rfl
theorem rrLegs_sing (τ : Type) (x : CoTickSing Tc (mem ℓ) τ) :
    rrLegs (mem := mem) (ℓ := ℓ) (.sing τ) x = x.rr := rfl
theorem rrLegs_stream (α : Type) (inst : DecidableEq α) (ord : StrOrd)
    (ret : Retries) (x : CoTickStream Tc (mem ℓ) α ord ret) :
    rrLegs (mem := mem) (ℓ := ℓ) (.stream α inst ord ret) x = x.rr := rfl
theorem Wf_pair (a b : TickShape) (x : TickedOf (Tk Tc mem) (TS Tc mem) ℓ (.pair a b)) :
    Wf (.pair a b) x = (Wf a x.1 ∧ Wf b x.2) := rfl
theorem Wf_sing (τ : Type) (x : CoTickSing Tc (mem ℓ) τ) :
    Wf (mem := mem) (ℓ := ℓ) (.sing τ) x = x.wf := rfl
theorem Wf_stream (α : Type) (inst : DecidableEq α) (ord : StrOrd)
    (ret : Retries) (x : CoTickStream Tc (mem ℓ) α ord ret) :
    Wf (mem := mem) (ℓ := ℓ) (.stream α inst ord ret) x = x.wf := rfl

theorem projSR_pair (a b : TickShape) (x : BoundedOf CoBSing CoBounded (.pair a b)) :
    projSR (.pair a b) x = (projSR a x.1, projSR b x.2) := rfl
theorem projSR_sing (τ : Type) (x : CoBSing τ) : projSR (.sing τ) x = x.sr := rfl
theorem projSR_stream (α : Type) (inst : DecidableEq α) (ord : StrOrd)
    (ret : Retries) (x : CoBounded α ord ret) :
    projSR (.stream α inst ord ret) x = x.sr := rfl
theorem projRR_pair (a b : TickShape) (x : BoundedOf CoBSing CoBounded (.pair a b)) :
    projRR (.pair a b) x = (projRR a x.1, projRR b x.2) := rfl
theorem projRR_sing (τ : Type) (x : CoBSing τ) : projRR (.sing τ) x = x.rr := rfl
theorem projRR_stream (α : Type) (inst : DecidableEq α) (ord : StrOrd)
    (ret : Retries) (x : CoBounded α ord ret) :
    projRR (.stream α inst ord ret) x = x.rr := rfl
theorem WfB_pair (a b : TickShape) (x : BoundedOf CoBSing CoBounded (.pair a b)) :
    WfB (.pair a b) x = (WfB a x.1 ∧ WfB b x.2) := rfl
theorem WfB_sing (τ : Type) (x : CoBSing τ) : WfB (.sing τ) x = x.wf := rfl
theorem WfB_stream (α : Type) (inst : DecidableEq α) (ord : StrOrd)
    (ret : Retries) (x : CoBounded α ord ret) :
    WfB (.stream α inst ord ret) x = x.wf := rfl

theorem embedSR_pair (a b : TickShape)
    (x : BoundedOf SchedTick.BS SchedTick.BStr (.pair a b)) :
    embedSR (.pair a b) x = (embedSR a x.1, embedSR b x.2) := rfl
theorem embedSR_sing_sr (τ : Type) (v : τ) : (embedSR (.sing τ) v).sr = v := rfl
theorem embedSR_sing_rr (τ : Type) (v : τ) : (embedSR (.sing τ) v).rr = v := rfl
theorem embedSR_sing_wf (τ : Type) (v : τ) : (embedSR (.sing τ) v).wf = False := rfl
theorem embedSR_stream_sr (α : Type) (inst : DecidableEq α) (ord : StrOrd)
    (ret : Retries) (l : List α) : (embedSR (.stream α inst ord ret) l).sr = l := rfl
theorem embedSR_stream_wf (α : Type) (inst : DecidableEq α) (ord : StrOrd)
    (ret : Retries) (l : List α) : (embedSR (.stream α inst ord ret) l).wf = False := rfl
theorem embedRR_pair (a b : TickShape)
    (x : BoundedOf ValuesTick.BS PoolCarrier (.pair a b)) :
    embedRR (.pair a b) x = (embedRR a x.1, embedRR b x.2) := rfl
theorem embedRR_sing_sr (τ : Type) (v : τ) : (embedRR (.sing τ) v).sr = v := rfl
theorem embedRR_sing_rr (τ : Type) (v : τ) : (embedRR (.sing τ) v).rr = v := rfl
theorem embedRR_sing_wf (τ : Type) (v : τ) : (embedRR (.sing τ) v).wf = False := rfl
theorem embedRR_stream_rr (α : Type) (inst : DecidableEq α) (ord : StrOrd)
    (ret : Retries) (p : PoolCarrier α ord ret) :
    (embedRR (.stream α inst ord ret) p).rr = p := rfl
theorem embedRR_stream_wf (α : Type) (inst : DecidableEq α) (ord : StrOrd)
    (ret : Retries) (p : PoolCarrier α ord ret) :
    (embedRR (.stream α inst ord ret) p).wf = False := rfl
theorem embed2_pair (a b : TickShape)
    (m : BoundedOf SchedTick.BS SchedTick.BStr (.pair a b))
    (v : BoundedOf ValuesTick.BS PoolCarrier (.pair a b)) :
    embed2 (.pair a b) m v = (embed2 a m.1 v.1, embed2 b m.2 v.2) := rfl
theorem embed2_sing_sr (τ : Type) (m v : τ) : (embed2 (.sing τ) m v).sr = m := rfl
theorem embed2_sing_rr (τ : Type) (m v : τ) : (embed2 (.sing τ) m v).rr = v := rfl
theorem embed2_sing_wf (τ : Type) (m v : τ) : (embed2 (.sing τ) m v).wf = (m = v) := rfl
theorem embed2_stream_sr (α : Type) (inst : DecidableEq α) (ord : StrOrd)
    (ret : Retries) (m : List α) (v : PoolCarrier α ord ret) :
    (embed2 (.stream α inst ord ret) m v).sr = m := rfl
theorem embed2_stream_rr (α : Type) (inst : DecidableEq α) (ord : StrOrd)
    (ret : Retries) (m : List α) (v : PoolCarrier α ord ret) :
    (embed2 (.stream α inst ord ret) m v).rr = v := rfl
theorem coerce_pair (a b : TickShape)
    (m : BoundedOf SchedTick.BS SchedTick.BStr (.pair a b)) :
    coerce (.pair a b) m = (coerce a m.1, coerce b m.2) := rfl
theorem coerce_sing (τ : Type) (v : τ) : coerce (.sing τ) v = v := rfl
/-- A list is coupled with its own coercion (the `W3` leaf fact at
stream inputs). -/
theorem coupledBatch_coerce (α : Type) (inst : DecidableEq α) (ord : StrOrd)
    (ret : Retries) (m : List α) :
    CoupledBatch ord ret m (coerce (.stream α inst ord ret) m) = True := by
  apply propext
  refine ⟨fun _ => trivial, fun _ => ?_⟩
  cases ord <;> cases ret <;> first | rfl | trivial

theorem allEO_sing (τ : Type) : allEO (.sing τ) = true := rfl
theorem allEO_stream_eo (α : Type) (inst : DecidableEq α) (ord : StrOrd) :
    allEO (.stream α inst ord .exactlyOnce) = true := rfl
theorem allEO_pair_eq (a b : TickShape) :
    allEO (.pair a b) = (allEO a && allEO b) := rfl

theorem gS_def (sts ins outs : TickShape)
    (g : Fin (mem ℓ) → BoundedOf CoBSing CoBounded sts → BoundedOf CoBSing CoBounded ins →
      BoundedOf CoBSing CoBounded sts × BoundedOf CoBSing CoBounded outs) :
    gS sts ins outs g = fun i s inp =>
      (projSR sts (g i (embedSR sts s) (embedSR ins inp)).1,
       projSR outs (g i (embedSR sts s) (embedSR ins inp)).2) := rfl
theorem gR_def (sts ins outs : TickShape)
    (g : Fin (mem ℓ) → BoundedOf CoBSing CoBounded sts → BoundedOf CoBSing CoBounded ins →
      BoundedOf CoBSing CoBounded sts × BoundedOf CoBSing CoBounded outs) :
    gR sts ins outs g = fun i s inp =>
      (projRR sts (g i (embedRR sts s) (embedRR ins inp)).1,
       projRR outs (g i (embedRR sts s) (embedRR ins inp)).2) := rfl
theorem gC_def (sts ins outs : TickShape)
    (g : Fin (mem ℓ) → BoundedOf CoBSing CoBounded sts → BoundedOf CoBSing CoBounded ins →
      BoundedOf CoBSing CoBounded sts × BoundedOf CoBSing CoBounded outs) :
    gC sts ins outs g = fun i s inp =>
      g i (embed2 sts s (coerce sts s)) (embed2 ins inp (coerce ins inp)) := rfl

end Rules

end CoTick

set_option warn.classDefReducibility false in
/-- Couple at `Tc`, derive at `Td` (`Tc ≤ Td`). Programs run at
`(T, T)`; the knots' graded couplings re-instantiate their generic
bodies at lowered couple horizons `(j, T)`. -/
def CoupleSem (L : Type) (mem : L → Nat)
    (pacing : (ℓ : L) → Fin (mem ℓ) → Nat → Bool)
    (Tc Td : Nat) (hjT : Tc ≤ Td) : HydroSem L mem where
  Stream ℓ α _ ord ret := CoStream Tc (mem ℓ) α ord ret
  KeyedStream p c α _ ord ret := CoKeyed Tc (mem p) (mem c) α ord ret
  Singleton ℓ α σ _ ord ret b := CoSing Tc (mem ℓ) α σ ord ret b
  Ticked ℓ σ := CoTickSing Tc (mem ℓ) σ
  -- one tick's content: both representations with their coupling
  BoundedStream α _ ord ret := CoBounded α ord ret
  BoundedSingleton σ := CoBSing σ
  -- overrides the signature default: the corner couples a machine
  -- tick-stream wire (step-indexed lists) with a denotation wire
  -- (quotient batches) — two legs with different history axes
  TickStream ℓ α _ ord ret := CoTickStream Tc (mem ℓ) α ord ret
  TransportDec p c := Fin p → Fin c → Nat → Nat
  OrderSelDec _ _ := Unit
  SnapDec _ _ _ := Unit
  BatchDec _ _ := Unit
  OrdBatchDec _ := Unit
  SampleDec n := SampleTimes n
  TimerDec n := TimerVerdicts n
  PulseDec n := TimingPulses n
  FixDec := Unit
  map s f :=
    { sr := fun i => (s.sr i).map (f i)
      rr := (Values L mem).map s.rr f
      wf := s.wf
      cpl := fun hwf i => ListLe.map_eo (f i) (s.cpl hwf i) }
  filterMap s f :=
    { sr := fun i => (s.sr i).filterMap (f i)
      rr := (Values L mem).filterMap s.rr f
      wf := s.wf
      cpl := fun hwf i => ListLe.filterMap_eo (f i) (s.cpl hwf i) }
  broadcast_closed dt s :=
    { sr := fun i j => (s.sr j).deliver (dt i j)
      rr := (Values L mem).broadcast_closed () s.rr
      wf := s.wf
      cpl := fun hwf i j =>
        deliver_view_cpl (s.sr j) (dt i j) Tc (s.cpl hwf j) }
  demux dt s addr :=
    { sr := fun i j => ((s.sr j).filterMap
        (fun dx => if dx.1 = addr i then some dx.2 else none)).deliver
        (dt i j)
      rr := (Values L mem).demux () s.rr addr
      wf := s.wf
      cpl := fun hwf i j =>
        deliver_view_cpl ((s.sr j).filterMap _) (dt i j) Tc
          (by
            refine ListLe.of_prefix (List.prefix_refl _) ?_
            exact ListLe.filterMap_eo _ (s.cpl hwf j)) }
  values k :=
    { sr := fun i => mergeN (k.sr i)
      rr := (Values L mem).values k.rr
      wf := k.wf
      cpl := fun hwf => corr_values (fun i j => k.cpl hwf i j) }
  weaken_retries s :=
    { sr := s.sr
      rr := (Values L mem).weaken_retries s.rr
      wf := s.wf
      cpl := fun hwf => corr_weaken (fun i => s.cpl hwf i) }
  union a b :=
    { sr := fun i => merge2 (a.sr i) (b.sr i)
      rr := (Values L mem).union a.rr b.rr
      wf := a.wf ∧ b.wf
      cpl := fun hwf =>
        corr_union (fun i => a.cpl hwf.1 i) (fun i => b.cpl hwf.2 i) }
  assume_ordering u _sel :=
    { sr := u.sr
      rr := (Values L mem).assume_ordering u.rr (ordSelDerive Td u.sr)
      wf := u.wf
      cpl := fun hwf i => by
        show (u.sr i).view Tc
          <+: selectOrder (u.rr i) (ordSelDerive Td u.sr i)
        obtain ⟨e, he⟩ := ordSelDerive_mono hjT u.sr i
        rw [← he]
        exact selectOrder_prefix_ext _ e _ (u.cpl hwf i) }
  fold g init ok s :=
    { sr := fun i => ⟨s.sr i, fun l => l.foldl g init⟩
      rr := (Values L mem).fold g init ok s.rr
      wf := s.wf
      cpl := fun hwf =>
        ⟨g, init, ok, s.rr, fun _i => rfl, fun _i => rfl,
         fun i => s.cpl hwf i⟩ }
  fold_monotone vo g init ok hinfl s :=
    { sr := fun i => ⟨s.sr i, fun l => l.foldl g init⟩
      rr := (Values L mem).fold_monotone vo g init ok hinfl s.rr
      wf := s.wf
      cpl := fun hwf =>
        ⟨g, init, ok, s.rr, fun _i => rfl, fun _i => rfl,
         fun i => s.cpl hwf i⟩ }
  snapshot {ℓ _α _σ _ ord ret b} s _cutd :=
    { sr := fun i t => (tickSteps (pacing ℓ i) t).map
        (fun st => (s.sr i).read ((s.sr i).src.view st))
      rr := (Values L mem).snapshot (ret := ret) s.rr
        (snapDerive ord ret (pacing ℓ) Td s.sr)
      wf := s.wf
      cpl := fun hwf k hk i => by
        obtain ⟨g, init, ok, pool, hread, hfold, hsrc⟩ := s.cpl hwf
        have hws : (tickSteps (pacing ℓ i) k).map
            (fun st => (s.sr i).read ((s.sr i).src.view st))
            = ((tickSteps (pacing ℓ i) k).map
                (fun st => (s.sr i).src.view st)).map
              (fun w => w.foldl g init) := by
          rw [List.map_map]
          exact List.map_congr_left (fun st _ => by rw [hfold i]; rfl)
        have heq := sched_snap_eq' ord ret g init ok (pool i)
          ((tickSteps (pacing ℓ i) k).map
            (fun st => (s.sr i).src.view st))
          (viewsChain _ (tickSteps_pairwise (pacing ℓ i) k))
          (fun w hw => by
            obtain ⟨st, hst, rfl⟩ := List.mem_map.mp hw
            exact ListLe.of_prefix
              ((s.sr i).src.mono_le ((tickSteps_le hst).trans hk))
              (hsrc i))
        have hext := snapTrace_mono_dec g init ok (pool i)
          (snapDerive_mono ord ret (pacing ℓ) (hk.trans hjT) s.sr i)
        have hgoal : (tickSteps (pacing ℓ i) k).map
            (fun st => (s.sr i).read ((s.sr i).src.view st))
            <+: singReads b s.rr i
              (snapDerive ord ret (pacing ℓ) Td s.sr i) := by
          rw [hws, heq, hread i]
          exact hext
        show _ <+: ((Values L mem).snapshot (ℓ := ℓ)
          (ord := ord) (ret := ret) s.rr
          (snapDerive ord ret (pacing ℓ) Td s.sr)) i
        rw [tickVals_snapshot]
        exact hgoal }
  batch {ℓ _α _} s _cutd :=
    { sr := fun i t => batchesFrom ((s.sr i).view)
        (tickSteps (pacing ℓ i) t) 0
      rr := (Values L mem).batch s.rr (batchDerive (pacing ℓ) Td s.sr)
      wf := s.wf
      cpl := fun hwf k hk i => by
        show (batchesFrom ((s.sr i).view) (tickSteps (pacing ℓ i) k)
            0).map (fun b => Multiset.ofList b)
          <+: batchCuts (s.rr i) 0 (batchDerive (pacing ℓ) Td s.sr i)
        have hreal : batchCuts (s.rr i) 0
            (batchDerive (pacing ℓ) k s.sr i)
            = batchDerive (pacing ℓ) k s.sr i :=
          corr_batch (pacing ℓ) k s.sr (fun j => s.rr j)
            (fun j => s.cpl_le hwf hk j) i
        have hmono := batchCuts_mono_dec (pool := s.rr i)
          (c := 0) (batchDerive_mono (pacing ℓ) (hk.trans hjT) s.sr i)
        rw [hreal] at hmono
        exact hmono }
  batch_ordered {ℓ _α _} s _cutd :=
    { sr := fun i t => batchesFrom ((s.sr i).view)
        (tickSteps (pacing ℓ i) t) 0
      rr := (Values L mem).batch_ordered s.rr
        (batchOrdDerive (pacing ℓ) Td s.sr)
      wf := s.wf
      cpl := fun hwf k hk i => by
        show batchesFrom ((s.sr i).view) (tickSteps (pacing ℓ i) k) 0
          <+: sliceCuts (s.rr i) 0 (batchOrdDerive (pacing ℓ) Td s.sr i)
        have hreal : sliceCuts (s.rr i) 0
            (batchOrdDerive (pacing ℓ) k s.sr i)
            = batchesFrom ((s.sr i).view) (tickSteps (pacing ℓ i) k) 0 :=
          corr_batch_ordered (pacing ℓ) k s.sr (fun j => s.rr j)
            (fun j => s.cpl_le hwf hk j) i
        have hmono := sliceCuts_mono_dec (pool := s.rr i)
          (c := 0) (batchOrdDerive_mono (pacing ℓ) (hk.trans hjT) s.sr i)
        rw [hreal] at hmono
        exact hmono }
  mapTick s f :=
    { sr := fun i step => (s.sr i step).map (f i)
      rr := (Values L mem).mapTick s.rr f
      wf := s.wf
      cpl := fun hwf k hk i =>
        List.IsPrefix.map (f i) (s.cpl hwf k hk i) }
  zipTick a b :=
    { sr := fun i step => Trace.zip (a.sr i step) (b.sr i step)
      rr := (Values L mem).zipTick a.rr b.rr
      wf := a.wf ∧ b.wf
      cpl := fun hwf k hk i =>
        zip_prefix (a.cpl hwf.1 k hk i) (b.cpl hwf.2 k hk i) }
  defer_tick init t :=
    { sr := fun i step => init :: t.sr i step
      rr := (Values L mem).defer_tick init t.rr
      wf := t.wf
      cpl := fun hwf k hk i =>
        List.cons_prefix_cons.mpr ⟨rfl, t.cpl hwf k hk i⟩ }
  sample_every t times :=
    { sr := famHist (fun step i => sampleAtOpt (t.sr i step)
        (times i))
      rr := (Values L mem).sample_every t.rr times
      wf := t.wf
      cpl := fun hwf i => by
        obtain ⟨k, hk, he⟩ := famFreeze_eq_raw
          (fun step i => sampleAtOpt (t.sr i step) (times i)) Tc
        show StutterSeq.le
          (StutterSeq.mk (famFreeze (fun step i =>
            sampleAtOpt (t.sr i step) (times i)) Tc i))
          (StutterSeq.mk (sampleAtOpt (t.rr i) (times i)))
        rw [congrFun he i]
        exact destutter_prefix
          (sampleAtOpt_prefix (t.cpl hwf k hk i) (times i)) }
  timeout_snapshot {ℓ _α _ _ord _ret} s verd :=
    { sr := fun i step => (verd i).take
        (tickSteps (pacing ℓ i) step).length
      rr := (Values L mem).timeout_snapshot s.rr verd
      wf := True
      cpl := fun _hwf _k _hk i => List.take_prefix _ _ }
  source_interval_batch {ℓ} pulses :=
    { sr := fun i step => (pulses i).take
        (tickSteps (pacing ℓ i) step).length
      rr := (Values L mem).source_interval_batch pulses
      wf := True
      cpl := fun _hwf _k _hk i => List.take_prefix _ _ }
  flattenOrdered t :=
    { sr := t.sr
      rr := (Values L mem).flattenOrdered t.rr
      wf := t.wf
      cpl := fun hwf k hk i => t.cpl hwf k hk i }
  flattenUnordered t :=
    { sr := t.sr
      rr := (Values L mem).flattenUnordered t.rr
      wf := t.wf
      cpl := fun hwf k hk i =>
        List.IsPrefix.map _ (t.cpl hwf k hk i) }
  allTicks {ℓ _β _ ord} bs :=
    { sr := famHist (fun step i => (bs.sr i step).flatten)
      rr := (Values L mem).allTicks bs.rr
      wf := bs.wf
      cpl := fun hwf i => by
        obtain ⟨k, hk, he⟩ := famFreeze_eq_raw
          (fun step i => (bs.sr i step).flatten) Tc
        have hview : famFreeze
            (fun step i => (bs.sr i step).flatten) Tc i
            = (bs.sr i k).flatten := congrFun he i
        cases ord with
        | totalOrder =>
          show famFreeze (fun step i => (bs.sr i step).flatten) Tc i
            <+: (bs.rr i).flatten
          rw [hview]
          exact prefix_flatten (bs.cpl hwf k hk i)
        | noOrder =>
          show Multiset.ofList (famFreeze
              (fun step i => (bs.sr i step).flatten) Tc i)
            ≤ (bs.rr i).sum
          rw [hview]
          exact flatten_le_sum (bs.cpl hwf k hk i) }
  fix_stream {ℓ α _ ord ret} _df body :=
    { sr := CoStream.fixSr body
      rr := iterate (CoStream.vbody body)
        (fun _i => PoolBot ord ret) (Td + 1)
      wf := (∀ (h : Nat) (x y : Fin (mem ℓ) → StepHist α),
              SAgree h x y →
              SAgree h (CoStream.sbody body x) (CoStream.sbody body y))
          ∧ (∀ m i, PoolLe ord ret
              (iterate (CoStream.vbody body)
                (fun _ => PoolBot ord ret) m i)
              (iterate (CoStream.vbody body)
                (fun _ => PoolBot ord ret) (m + 1) i))
          ∧ (∀ j, j ≤ Tc → ∀ (x : Fin (mem ℓ) → StepHist α)
              (v : Fin (mem ℓ) → PoolCarrier α ord ret),
              (∀ t, t ≤ j → ∀ i, ListLe ord ret ((x i).view t) (v i)) →
              ∀ i, ListLe ord ret ((CoStream.sbody body x i).view j)
                ((body (CoStream.probe2 x v)).rr i))
      cpl := fun hwf i => by
        obtain ⟨hcaus, hchain, hcplj⟩ := hwf
        refine ListLe.trans_pool
          (co_fix_cpl body hcaus hcplj Tc (Nat.le_refl _) i) ?_
        refine pool_chain_glue (f := fun m =>
          iterate (CoStream.vbody body)
            (fun _ => PoolBot ord ret) m i)
          (a := Tc + 1) (b := Td + 1) (by omega) ?_
        intro m _hm _hmb
        exact hchain m i }
  fix_tick {ℓ σ} _df body :=
    { sr := CoTickSing.fixSr body
      rr := iterate (CoTickSing.vbody body)
        (fun _i => ([] : Trace σ)) (Td + 1)
      wf := (∀ (h : Nat) (x y : Fin (mem ℓ) → Nat → Trace σ),
              TAgree h x y →
              TAgree h (CoTickSing.sbody body x)
                (CoTickSing.sbody body y))
          ∧ (∀ m i,
              (iterate (CoTickSing.vbody body)
                (fun _ => ([] : Trace σ)) m i)
              <+: (iterate (CoTickSing.vbody body)
                (fun _ => ([] : Trace σ)) (m + 1) i))
          ∧ (∀ j, j ≤ Tc → ∀ (x : Fin (mem ℓ) → Nat → Trace σ)
              (v : TickV (mem ℓ) σ),
              (∀ t, t ≤ j → ∀ i, x i t <+: v i) →
              ∀ i, CoTickSing.sbody body x i j
                <+: (body (CoTickSing.probe2 x v)).rr i)
      cpl := fun hwf k hk i => by
        obtain ⟨hcaus, hchain, hcplj⟩ := hwf
        refine (co_tick_fix_cpl body hcaus hchain hcplj k hk i).trans ?_
        refine trace_chain_glue (f := fun m =>
          iterate (CoTickSing.vbody body)
            (fun _ => ([] : Trace σ)) m i)
          (a := k + 1) (b := Td + 1) (by omega) ?_
        intro m _hm _hmb
        exact hchain m i }

  tick_scan sts ins outs x g init := CoTick.scan pacing sts ins outs x g init
  -- in-tick operators: legwise, transporting the per-tick coupling;
  -- plain-typed results from the machine leg (their denotation
  -- agreement is the conditional co-rule in CoupleProj)
  bmap b f :=
    { sr := b.sr.map f, rr := (Values L mem).bmap b.rr f, wf := b.wf
      cpl := fun h => CoupledBatch.map (b.cpl h) f }
  bfilterMap b f :=
    { sr := b.sr.filterMap f, rr := (Values L mem).bfilterMap b.rr f
      wf := b.wf, cpl := fun h => CoupledBatch.filterMap (b.cpl h) f }
  bflatMapOrdered l f :=
    { sr := l.sr.flatMap f, rr := (Values L mem).bflatMapOrdered l.rr f
      wf := l.wf
      cpl := fun h => by
        show l.sr.flatMap f = l.rr.flatMap f
        rw [show l.sr = l.rr from l.cpl h] }
  bflatMapUnordered b f :=
    { sr := b.sr.flatMap (fun a => (f a).sr)
      rr := (Values L mem).bflatMapUnordered b.rr (fun a => (f a).rr)
      wf := b.wf ∧ ∀ a ∈ b.sr, (f a).wf
      cpl := fun h => CoupledBatch.flatMapUnordered (b.cpl h.1)
        (fun a => (f a).sr) (fun a => (f a).rr)
        (fun a ha => (f a).cpl (h.2 a ha)) }
  bofList l :=
    { sr := l, rr := (Values L mem).bofList l, wf := True
      cpl := fun _ => rfl }
  bcount b :=
    { sr := b.sr.length, rr := (Values L mem).bcount b.rr, wf := b.wf
      cpl := fun h => CoupledBatch.count (b.cpl h) }
  -- the in-tick fold is coupled at `ExactlyOnce` (the retry grades
  -- carry no per-tick coupling — no tick stream is born at them)
  bfold {_α _σ _ _ord ret} g init ok b :=
    { sr := b.sr.foldl g init, rr := (Values L mem).bfold g init ok b.rr
      wf := b.wf ∧ ret = .exactlyOnce
      cpl := fun h => CoupledBatch.fold g init ok (b.cpl h.1) h.2 }
  benumerate l :=
    { sr := listEnumerate l.sr, rr := (Values L mem).benumerate l.rr
      wf := l.wf
      cpl := fun h => by
        show listEnumerate l.sr = listEnumerate l.rr
        rw [show l.sr = l.rr from l.cpl h] }
  bfirst l :=
    { sr := l.sr.head?, rr := (Values L mem).bfirst l.rr, wf := l.wf
      cpl := fun h => by
        show l.sr.head? = l.rr.head?
        rw [show l.sr = l.rr from l.cpl h] }
  bcrossSingleton b s :=
    { sr := b.sr.map (fun a => (a, s.sr))
      rr := (Values L mem).bcrossSingleton b.rr s.rr, wf := b.wf ∧ s.wf
      cpl := fun h => by
        rw [s.cpl h.2]
        exact CoupledBatch.map (b.cpl h.1) (fun a => (a, s.rr)) }
  bchain a b :=
    { sr := a.sr ++ b.sr, rr := (Values L mem).bchain a.rr b.rr
      wf := a.wf ∧ b.wf
      cpl := fun h => CoupledBatch.chain (a.cpl h.1) (b.cpl h.2) }
  bweakenOrder b :=
    { sr := b.sr, rr := (Values L mem).bweakenOrder b.rr, wf := b.wf
      cpl := fun h => CoupledBatch.weakenOrder (b.cpl h) }
  bfilter b p :=
    { sr := b.sr.filter p, rr := (Values L mem).bfilter b.rr p, wf := b.wf
      cpl := fun h => CoupledBatch.filter (b.cpl h) p }
  bkeyedFold g init ok b :=
    { sr := keyedFoldList g init b.sr
      rr := (Values L mem).bkeyedFold g init ok b.rr, wf := b.wf
      cpl := fun h => CoupledBatch.keyedFold g init ok (b.cpl h) }
  bkeys e :=
    { sr := e.sr.map Prod.fst, rr := (Values L mem).bkeys e.rr, wf := e.wf
      cpl := fun h => CoupledBatch.map (ord := .noOrder) (e.cpl h) Prod.fst }
  bjoin a b :=
    { sr := listJoin a.sr b.sr, rr := (Values L mem).bjoin a.rr b.rr
      wf := a.wf ∧ b.wf
      cpl := fun h => CoupledBatch.join (a.cpl h.1) (b.cpl h.2) }
  bantiJoin a ks :=
    { sr := a.sr.filter (fun e => !(decide (e.1 ∈ ks.sr)))
      rr := (Values L mem).bantiJoin a.rr ks.rr
      wf := a.wf ∧ ks.wf
      cpl := fun h => CoupledBatch.antiJoin (a.cpl h.1) (ks.cpl h.2) }
  bfilterNotIn a o :=
    { sr := a.sr.filter (fun x => !(decide (x ∈ o.sr)))
      rr := (Values L mem).bfilterNotIn a.rr o.rr
      wf := a.wf ∧ o.wf
      cpl := fun h => CoupledBatch.filterNotIn (a.cpl h.1) (o.cpl h.2) }
  bmax b :=
    { sr := b.sr.foldl maxStep none, rr := (Values L mem).bmax b.rr, wf := b.wf
      cpl := fun h => CoupledBatch.max (b.cpl h) }
  bfilterIf b flag :=
    { sr := if flag.sr then b.sr else []
      rr := (Values L mem).bfilterIf b.rr flag.rr
      wf := b.wf ∧ flag.wf
      cpl := fun h => by
        rw [flag.cpl h.2]
        exact CoupledBatch.filterIf (b.cpl h.1) flag.rr }
  bsPure v := CoBSing.pure v
  bsMap s f := CoBSing.map s f
  bsZip a b := CoBSing.zip a b
  boMap o f := CoBSing.map o (fun x => x.map f)
  boUnwrapOr o s := CoBSing.unwrapOr o s
  boFilter o p := CoBSing.map o (fun x => x.filter p)
  boIsSome o := CoBSing.map o (fun x => x.isSome)

end Hydro
