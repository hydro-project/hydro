import Hydro.Transfer
import Hydro.SchedCausal
import Hydro.SchedMono

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
  rr : MemberMap n (PoolCarrier α ord ret)
  wf : Prop
  cpl : wf → ∀ i, ListLe ord ret ((sr i).view T) (rr i)

/-- Fold corner: the machine's live value per step against the
denotational read function. The coupling is **read agreement**: the
machine's value is some read `rd` of the corner's remembered source
(`src` — the machine wire the fold consumes, kept by the corner only to
derive the denotational cut decisions; the machine carrier itself holds
just the value), the source sits below a pool at `T`, and reading an
ascending chain of source views equals the denotational reads at the
derived cuts (`snapCut`), which extend under cut extension. For a plain
fold this is `sched_snap_eq'` + `snapTrace_mono_dec`; a keyed-sourced
singleton (`get_max_key`) brings its own agreement lemma — the package
is exactly what `snapshot` consumes. -/
structure CoSing (T : Nat) (n : Nat) (α σ : Type) [DecidableEq α]
    (ord : StrOrd) (ret : Retries) (b : SingBound σ) : Type where
  sr : Fin n → Nat → σ
  rr : SingletonV n α σ ord b
  src : Fin n → StepHist α
  wf : Prop
  cpl : wf → ∃ (rd : Fin n → List α → σ),
    (∀ i t, sr i t = rd i ((src i).view t)) ∧
    (∀ i (ws : List (List α)), List.IsChain (· <+: ·) ([] :: ws) →
      (∀ w ∈ ws, w <+: (src i).view T) →
      ws.map (rd i) = singReads b rr i (snapCut ord ret ws)) ∧
    (∀ i d d', CutLe ord d d' → singReads b rr i d <+: singReads b rr i d')

/-- Keyed-stream corner: the machine's flat wire of pairs against the
denotational keyed pool, coupled per key at the horizon. -/
structure CoKStream (T : Nat) (n : Nat) (K V : Type) [DecidableEq K] [DecidableEq V]
    (ord : StrOrd) (ret : Retries) : Type where
  sr : Fin n → StepHist (K × V)
  rr : MemberMap n (KeyedPool K V ord ret)
  wf : Prop
  cpl : wf → ∀ i, KeyedListLe ord ret ((sr i).view T) (rr i)

/-- Keyed-singleton corner (a keyed fold's live map): read agreement as
for `CoSing`, up to the list-vs-map coupling of one tick's map. -/
structure CoKSing (T : Nat) (n : Nat) (α K V : Type) [DecidableEq α] [DecidableEq K]
    [DecidableEq V] (ord : StrOrd) (ret : Retries) (b : KSBound) : Type where
  sr : Fin n → Nat → SchedKMap K V
  rr : MemberMap n (CutDec α .noOrder → Trace (KeyedMap K V))
  src : Fin n → StepHist α
  wf : Prop
  cpl : wf → ∃ (rd : Fin n → List α → SchedKMap K V),
    (∀ i t, sr i t = rd i ((src i).view t)) ∧
    (∀ i (ws : List (List α)), List.IsChain (· <+: ·) ([] :: ws) →
      (∀ w ∈ ws, w <+: (src i).view T) →
      List.Forall₂ (fun l m => KeyedMap.ofAssoc l = m) (ws.map (rd i)) (rr i (cutsMS 0 ws))) ∧
    (∀ i d d', CutLe .noOrder d d' → rr i d <+: rr i d')

/-- Atomic corner: coupled at every step `≤ T`. -/
structure CoTickSing (T : Nat) (n : Nat) (σ : Type) : Type where
  sr : Fin n → Nat → Trace σ
  rr : TickV n σ
  wf : Prop
  cpl : wf → ∀ k, k ≤ T → ∀ i, sr i k <+: rr i
  /-- The machine leg is step-monotone (a realized tick never changes;
  `SchedMono.lean`) — recorded in the corner, where the proofs live,
  so a per-tick order derived at `Td` is the order the machine held at
  every coupled step (D71). -/
  mono : wf → TMono sr

/-- Tick-stream corner: coupled at every step `≤ T`. -/
structure CoAtomicStream (T : Nat) (n : Nat) (α : Type) [DecidableEq α]
    (ord : StrOrd) (ret : Retries) : Type where
  sr : Fin n → Nat → Trace (List α)
  rr : MemberMap n (Trace (PoolCarrier α ord ret))
  wf : Prop
  cpl : wf → ∀ k, k ≤ T → ∀ i, BatchTraceLe ord ret (sr i k) (rr i)
  mono : wf → TMono sr

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

/-- One tick's keyed stream at the corner: the machine's flat pairs, the
denotation's keyed pool, coupled by grouping. -/
structure CoBKStream (K V : Type) [DecidableEq K] [DecidableEq V] (ord : StrOrd)
    (ret : Retries) : Type where
  sr : List (K × V)
  rr : KeyedPool K V ord ret
  wf : Prop
  cpl : wf → CoupledKeyed ord ret sr rr

/-- One tick's keyed singleton at the corner: the machine's association
list, the denotation's map, coupled by reading the list as a map. -/
structure CoBKSing (K V : Type) [DecidableEq K] [DecidableEq V] : Type where
  sr : SchedKMap K V
  rr : KeyedMap K V
  wf : Prop
  cpl : wf → KeyedMap.ofAssoc sr = rr

/-- Keyed-stream wire corner (the corner's `AtomicKeyedStream`): coupled
at every step `≤ T`, grouping per tick. -/
structure CoAtomicKStream (T : Nat) (n : Nat) (K V : Type) [DecidableEq K] [DecidableEq V]
    (ord : StrOrd) (ret : Retries) : Type where
  sr : Fin n → Nat → Trace (List (K × V))
  rr : MemberMap n (Trace (KeyedPool K V ord ret))
  wf : Prop
  cpl : wf → ∀ k, k ≤ T → ∀ i, KBatchTraceLe ord ret (sr i k) (rr i)
  mono : wf → TMono sr

/-- Keyed-singleton wire corner: coupled at every step `≤ T` (the
corner's `AtomicKeyedSingleton`). -/
structure CoAtomicKSing (T : Nat) (n : Nat) (K V : Type) [DecidableEq K] [DecidableEq V] :
    Type where
  sr : Fin n → Nat → Trace (SchedKMap K V)
  rr : MemberMap n (Trace (KeyedMap K V))
  wf : Prop
  cpl : wf → ∀ k, k ≤ T → ∀ i, (sr i k).map KeyedMap.ofAssoc <+: rr i
  mono : wf → TMono sr

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

/-- `into_keyed` inside the tick: a coupled batch, grouped. -/
theorem intoKeyed {K V : Type} [DecidableEq K] [DecidableEq V] {ord : StrOrd}
    {s : List (K × V)} {v : PoolCarrier (K × V) ord .exactlyOnce}
    (h : CoupledBatch ord .exactlyOnce s v) :
    CoupledKeyed ord .exactlyOnce s (KeyedPool.ofPool v) :=
  CoupledKeyed.ofPool_batch (by cases ord <;> exact h)

/-- The keyed `.keys()`: the association list's keys are the map's. -/
theorem ksKeys {K V : Type} [DecidableEq K] [DecidableEq V]
    {s : SchedKMap K V} {m : KeyedMap K V} (h : KeyedMap.ofAssoc s = m) :
    CoupledBatch .noOrder .exactlyOnce s.keys m.keysMS := by
  show (↑s.keys : Multiset K) = m.keysMS
  rw [KeyedMap.coe_keys_ofAssoc, h]

/-- The keyed `.entries()`. -/
theorem ksEntries {K V : Type} [DecidableEq K] [DecidableEq V]
    {s : SchedKMap K V} {m : KeyedMap K V} (h : KeyedMap.ofAssoc s = m) :
    CoupledBatch .noOrder .exactlyOnce s.entries m.entries := by
  show (↑s.entries : Multiset (K × V)) = m.entries
  rw [KeyedMap.coe_entries_ofAssoc, h]

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
    (init : σ) (Pre : α → α → Prop) [DecidableRel Pre] (ok : FoldOk ord ret Pre g) {s : List α}
    {v : PoolCarrier α ord ret} (h : CoupledBatch ord ret s v)
    (hret : ret = .exactlyOnce) (hpre : PoolPairwise ord ret Pre v) :
    s.foldl g init = PoolFold ord ret Pre g init ok v := by
  subst hret
  cases ord
  · exact congrArg (List.foldl g init) h
  · show s.foldl g init = Multiset.foldlOn Pre g init ok v
    rw [← h]
    exact (Multiset.foldlOn_coe Pre g init ok (PoolPairwise.of_coe_le (le_of_eq h) hpre)).symm

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
  rr := .mk' fun _i => PoolBot ord ret
  wf := False
  cpl := fun hwf => absurd hwf (by simp)

/-- External input: a machine wire coupled to a denotational pool at
every horizon (the program-boundary hypothesis). -/
def input (h : Fin n → StepHist α) (v : MemberMap n (PoolCarrier α ord ret))
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
def probe2 (x : Fin n → StepHist α) (v : MemberMap n (PoolCarrier α ord ret)) :
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

variable (p : Fin n → Nat → Bool) (body : CoStream T n α ord ret → CoStream T n α ord ret)

/-- The machine-side body. -/
def sbody (x : Fin n → StepHist α) : Fin n → StepHist α :=
  (body (schedEmbed x)).sr

/-- The knot's machine leg: the frozen diagonal of the machine body's
shift-iterates at the members' tick skeletons `p` (the `SchedSem` fix
semantics, verbatim). -/
def fixSr : Fin n → StepHist α :=
  famHist (fun t i =>
    ((iterate (fun x j => ((sbody body x) j).shift (p j))
      (fun _ => StepHist.bot) (t + 1)) i).view t)

/-- The reader-side body at the knot's own machine diagonal: sites
inside the loop derive their decisions from the same machine wire the
closed knot presents to the rest of the program. -/
def vbody (v : MemberMap n (PoolCarrier α ord ret)) :
    MemberMap n (PoolCarrier α ord ret) :=
  (body (probe2 (fixSr p body) v)).rr

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
  src := C.src
  wf := C.wf
  cpl := fun hwf => by
    obtain ⟨rd, hval, hagree, hmono⟩ := C.cpl hwf
    exact ⟨rd, hval, fun i ws hchain hpre => hagree i ws hchain
      (fun w hw => (hpre w hw).trans ((C.src i).mono_le h)), hmono⟩

namespace CoTickSing

variable {T : Nat} {n : Nat} {σ : Type}

def schedEmbed (x : Fin n → Nat → Trace σ) :
    CoTickSing T n σ where
  sr := x
  rr := .mk' fun _i => ([] : Trace σ)
  wf := False
  cpl := fun hwf => absurd hwf (by simp)
  mono := fun hwf => absurd hwf (by simp)

/-- External input (tick-singleton boundary): a step-monotone machine
trace below a denotational trace. -/
def input (s : Fin n → Nat → Trace σ) (v : TickV n σ)
    (hc : ∀ t i, s i t <+: v i) (hm : TMono s) : CoTickSing T n σ where
  sr := s
  rr := v
  wf := True
  cpl := fun _ k _hk i => hc k i
  mono := fun _ => hm

/-- Two-leg probe (tick). -/
def probe2 (x : Fin n → Nat → Trace σ) (v : TickV n σ) :
    CoTickSing T n σ where
  sr := x
  rr := v
  wf := False
  cpl := fun hwf => absurd hwf (by simp)
  mono := fun hwf => absurd hwf (by simp)

/-- Lower the couple horizon (tick couplings quantify all steps `≤ T`,
so restriction is inclusion). -/
def lower (C : CoTickSing T n σ) {T' : Nat} (h : T' ≤ T) :
    CoTickSing T' n σ where
  sr := C.sr
  rr := C.rr
  wf := C.wf
  cpl := fun hwf k hk i => C.cpl hwf k (hk.trans h) i
  mono := C.mono

variable (p : Fin n → Nat → Bool) (body : CoTickSing T n σ →
  CoTickSing T n σ)

def sbody (x : Fin n → Nat → Trace σ) : Fin n → Nat → Trace σ :=
  (body (schedEmbed x)).sr

/-- The machine-side body with the tick shift (gated by the members'
tick skeletons `p`). -/
def sshift (x : Fin n → Nat → Trace σ) : Fin n → Nat → Trace σ :=
  fun j => gateView (p j) ((sbody body x) j)

/-- The knot's machine leg: the raw tick diagonal. -/
def fixSr : Fin n → Nat → Trace σ :=
  fun i t => (iterate (sshift p body) (fun _ _ => ([] : Trace σ)) (t + 1)) i t

def vbody (v : TickV n σ) : TickV n σ :=
  (body (probe2 (fixSr p body) v)).rr

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

variable (p : Fin n → Nat → Bool)
  (hcaus : ∀ (h : Nat) (x y : Fin n → StepHist α), SAgree h x y →
    SAgree h (CoStream.sbody body x) (CoStream.sbody body y))

include hcaus

/-- The raw diagonal ascends step by step (causality). -/
theorem co_diag_ascent : ∀ (t : Nat) (i : Fin n),
    ((iterate (fun x j => ((CoStream.sbody body x) j).shift (p j))
      (fun _ => StepHist.bot) (t + 1)) i).view t
    <+: ((iterate (fun x j => ((CoStream.sbody body x) j).shift (p j))
      (fun _ => StepHist.bot) (t + 2)) i).view (t + 1) := by
  intro t i
  calc ((iterate (fun x j => ((CoStream.sbody body x) j).shift (p j))
        (fun _ => StepHist.bot) (t + 1)) i).view t
      = ((iterate (fun x j => ((CoStream.sbody body x) j).shift (p j))
        (fun _ => StepHist.bot) (t + 2)) i).view t :=
      stab_iterate_shift (h := t) p
        (fun h' _hh' x y hxy => hcaus h' x y hxy)
        t (t + 1) (t + 2) (Nat.le_refl _)
        (Nat.lt_succ_self _) (by omega) i
    _ <+: _ := ((iterate (fun x j => ((CoStream.sbody body x) j).shift (p j))
        (fun _ => StepHist.bot) (t + 2)) i).mono t

/-- The frozen diagonal *is* the raw diagonal (causality ⇒ ascent ⇒
transparency). -/
theorem co_fix_view (t : Nat) (i : Fin n) :
    (CoStream.fixSr p body i).view t
      = ((iterate (fun x j => ((CoStream.sbody body x) j).shift (p j))
        (fun _ => StepHist.bot) (t + 1)) i).view t := by
  show famFreeze (fun t' j =>
      ((iterate (fun x j' => ((CoStream.sbody body x) j').shift (p j'))
        (fun _ => StepHist.bot) (t' + 1)) j).view t') t i = _
  rw [co_famFreeze_eq_of_ascending _ t
    (fun t' _ht' j => co_diag_ascent body p hcaus t' j)]

/-- The stage-`t+1` body output at `t` is the body's output at the closed knot. -/
theorem co_fix_stage_eq (t : Nat) (i : Fin n) :
    ((CoStream.sbody body
      (iterate (fun x j => ((CoStream.sbody body x) j).shift (p j))
        (fun _ => StepHist.bot) (t + 1))) i).view t
    = ((CoStream.sbody body (CoStream.fixSr p body)) i).view t := by
  refine hcaus t _ _ (fun j t' ht' => ?_) i t (Nat.le_refl _)
  rw [co_fix_view body p hcaus t' j]
  exact stab_iterate_shift (h := t') p
    (fun h' _hh' x y hxy => hcaus h' x y hxy)
    t' (t + 1) (t' + 1) (Nat.le_refl _)
    (by omega) (Nat.lt_succ_self _) j

/-- **The machine fixpoint inequation**: the knot's next view is a prefix
of the body's view at the closed knot (equal at a ticking step; at a
non-ticking step the knot holds its previous view, itself such a
prefix). -/
theorem co_fix_view_succ (t : Nat) (i : Fin n) :
    (CoStream.fixSr p body i).view (t + 1)
      <+: ((CoStream.sbody body (CoStream.fixSr p body)) i).view t := by
  rw [co_fix_view body p hcaus (t + 1) i]
  show gateView (p i) (CoStream.sbody body
      (iterate (fun x j => ((CoStream.sbody body x) j).shift (p j))
        (fun _ => StepHist.bot) (t + 1)) i).view (t + 1) <+: _
  rw [← co_fix_stage_eq body p hcaus t i, gateView_succ]
  split
  · exact List.prefix_refl _
  · exact gateView_prefix (p i) _ (fun u => (CoStream.sbody body _ i).mono u) t

/-- **The knot coupling**, by induction on the horizon. -/
theorem co_fix_cpl
    (hcplj : ∀ j, j ≤ T → ∀ (x : Fin n → StepHist α),
      (∀ t, t ≤ j → ∀ i, ListLe ord ret ((x i).view t)
        (iterate (CoStream.vbody p body) (.mk' fun _ => PoolBot ord ret) (j + 1) i)) →
      ∀ i, ListLe ord ret ((CoStream.sbody body x i).view j)
        ((body (CoStream.probe2 x
          (iterate (CoStream.vbody p body) (.mk' fun _ => PoolBot ord ret) (j + 1)))).rr i)) :
    ∀ j, j ≤ T → ∀ i, ListLe ord ret ((CoStream.fixSr p body i).view j)
      (iterate (CoStream.vbody p body)
        (.mk' fun _ => PoolBot ord ret) (j + 1) i)
  | 0, _hj, i => by
    rw [co_fix_view body p hcaus 0 i]
    exact ListLe.nil _ _ _
  | j + 1, hj, i => by
    refine ListLe.of_prefix (co_fix_view_succ body p hcaus j i) ?_
    refine hcplj j (by omega) _ (fun t ht i' => ?_) i
    refine ListLe.of_prefix ((CoStream.fixSr p body i').mono_le ht) ?_
    exact co_fix_cpl hcplj j (by omega) i'

end KnotCpl

/-! ## The knot coupling (tick) -/

section TickKnotCpl

variable {T : Nat} {n : Nat} {σ : Type} (p : Fin n → Nat → Bool)
  (body : CoTickSing T n σ → CoTickSing T n σ)
  (hcaus : ∀ (h : Nat) (x y : Fin n → Nat → Trace σ), TAgree h x y →
    TAgree h (CoTickSing.sbody body x) (CoTickSing.sbody body y))

include hcaus

/-- The stage-`t+1` body entry at `t` is the body's entry at the closed knot. -/
theorem co_tick_fix_stage_eq (t : Nat) (i : Fin n) :
    CoTickSing.sbody body
      (iterate (CoTickSing.sshift p body) (fun _ _ => ([] : Trace σ)) (t + 1)) i t
    = CoTickSing.sbody body (CoTickSing.fixSr p body) i t := by
  refine hcaus t _ _ (fun j t' ht' => ?_) i t (Nat.le_refl _)
  show (iterate (CoTickSing.sshift p body)
      (fun _ _ => ([] : Trace σ)) (t + 1)) j t' = CoTickSing.fixSr p body j t'
  exact stab_iterate_tick (h := t') p
    (fun h' _hh' x y hxy => hcaus h' x y hxy)
    t' (t + 1) (t' + 1) (Nat.le_refl _)
    (by omega) (Nat.lt_succ_self _) j

/-- The knot's machine leg is step-monotone when the body preserves
monotonicity (W4; `tmono_fix_tick` on the raw diagonal). -/
theorem co_tick_fix_mono
    (hmono : ∀ x, TMono x → TMono (CoTickSing.sbody body x)) :
    TMono (CoTickSing.fixSr p body) :=
  tmono_tick_diag (A := CoTickSing.sbody body) p hcaus hmono

/-- The tick fixpoint inequation (raw diagonal, no freeze): the knot's
next entry is a prefix of the body's entry at the closed knot. -/
theorem co_tick_fix_succ (hmono : ∀ x, TMono x → TMono (CoTickSing.sbody body x))
    (t : Nat) (i : Fin n) :
    CoTickSing.fixSr p body i (t + 1)
      <+: CoTickSing.sbody body (CoTickSing.fixSr p body) i t := by
  show gateView (p i) (CoTickSing.sbody body
      (iterate (CoTickSing.sshift p body) (fun _ _ => ([] : Trace σ)) (t + 1)) i) (t + 1) <+: _
  rw [← co_tick_fix_stage_eq p body hcaus t i, gateView_succ]
  split
  · exact List.prefix_refl _
  · have hm := hmono _ (TMono.tickshift p (A := CoTickSing.sbody body) hmono (t + 1))
    exact gateView_prefix (p i) _ (fun u => hm i u (u + 1) (Nat.le_succ u)) t

/-- The tick knot coupling (the ascent hypothesis chains earlier
horizons' couplings up to the current iterate — tick entries are not
views of one history, so the chain is explicit). The graded coupling
`hcplj` may assume its loop input step-monotone (it is: the machine leg
is the diagonal, `co_tick_fix_mono`). -/
theorem co_tick_fix_cpl
    (hchain : ∀ m i,
      (iterate (CoTickSing.vbody p body)
        (.mk' fun _ => ([] : Trace σ)) m i)
      <+: (iterate (CoTickSing.vbody p body)
        (.mk' fun _ => ([] : Trace σ)) (m + 1) i))
    (hcplj : ∀ j, j ≤ T → ∀ (x : Fin n → Nat → Trace σ),
      (∀ t, t ≤ j → ∀ i, x i t
        <+: iterate (CoTickSing.vbody p body) (.mk' fun _ => ([] : Trace σ)) (j + 1) i) →
      TMono x →
      ∀ i, CoTickSing.sbody body x i j
        <+: (body (CoTickSing.probe2 x
          (iterate (CoTickSing.vbody p body) (.mk' fun _ => ([] : Trace σ)) (j + 1)))).rr i)
    (hmono : ∀ x, TMono x → TMono (CoTickSing.sbody body x)) :
    ∀ j, j ≤ T → ∀ i, CoTickSing.fixSr p body i j
      <+: (iterate (CoTickSing.vbody p body)
        (.mk' fun _ => ([] : Trace σ)) (j + 1) i)
  | 0, _hj, i => by
    show (iterate (CoTickSing.sshift p body)
      (fun _ _ => ([] : Trace σ)) 1) i 0 <+: _
    exact List.nil_prefix
  | j + 1, hj, i => by
    refine (co_tick_fix_succ p body hcaus hmono j i).trans ?_
    refine hcplj j (by omega) _ (fun t ht i' => ?_) (co_tick_fix_mono p body hcaus hmono) i
    refine (co_tick_fix_cpl hchain hcplj hmono t (by omega) i').trans ?_
    refine trace_chain_glue (f := fun m =>
      iterate (CoTickSing.vbody p body)
        (.mk' fun _ => ([] : Trace σ)) m i')
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
    {ret : Retries} (x : StepHist α) (p : Nat → Bool) (cur : Nat → Nat)
    {v : PoolCarrier α ord ret} :
    ∀ T, ListLe ord ret (x.view T) v →
      ListLe ord ret ((x.deliver p cur).view T) v
  | 0, _ => ListLe.nil _ _ _
  | u + 1, hsrc => by
    show ListLe ord ret (gateView p _ (u + 1)) v
    rw [gateView_succ]
    split
    · exact ListLe.take _ (ListLe.of_prefix (x.mono_le (Nat.le_succ u)) hsrc)
    · exact deliver_view_cpl x p cur u (ListLe.of_prefix (x.mono_le (Nat.le_succ u)) hsrc)

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
    (ord : StrOrd) (ret : Retries) : Type := CoAtomicStream Tc (mem ℓ) α ord ret
abbrev TKS (Tc : Nat) (mem : L → Nat) (ℓ : L) (K V : Type) [DecidableEq K] [DecidableEq V] :
    Type := CoAtomicKSing Tc (mem ℓ) K V

/-- Every stream leaf is `ExactlyOnce`. -/
def allEO : TickShape → Bool
  | .sing _ => true
  | .stream _ _ _ ret => match ret with | .exactlyOnce => true | .atLeastOnce => false
  | .ksing _ _ _ _ => true
  | .pair a b => allEO a && allEO b

/-! ### Wires: the legs, the well-formedness, the coupling -/

def srLegs {ℓ : L} : (sh : TickShape) → AtomicOf (Tk Tc mem) (TS Tc mem) (TKS Tc mem) ℓ sh →
    AtomicOf (SchedTick.Tk mem) (SchedTick.TS mem) (SchedTick.TKS mem) ℓ sh
  | .sing _, x => x.sr
  | .stream _ _ _ _, x => x.sr
  | .ksing _ _ _ _, x => x.sr
  | .pair a b, x => (srLegs a x.1, srLegs b x.2)

def rrLegs {ℓ : L} : (sh : TickShape) → AtomicOf (Tk Tc mem) (TS Tc mem) (TKS Tc mem) ℓ sh →
    AtomicOf (ValuesTick.Tk mem) (ValuesTick.TS mem) (ValuesTick.TKS mem) ℓ sh
  | .sing _, x => x.rr
  | .stream _ _ _ _, x => x.rr
  | .ksing _ _ _ _, x => x.rr
  | .pair a b, x => (rrLegs a x.1, rrLegs b x.2)

def Wf {ℓ : L} : (sh : TickShape) → AtomicOf (Tk Tc mem) (TS Tc mem) (TKS Tc mem) ℓ sh → Prop
  | .sing _, x => x.wf
  | .stream _ _ _ _, x => x.wf
  | .ksing _ _ _ _, x => x.wf
  | .pair a b, x => Wf a x.1 ∧ Wf b x.2

theorem cpl {ℓ : L} : (sh : TickShape) → (x : AtomicOf (Tk Tc mem) (TS Tc mem) (TKS Tc mem) ℓ sh) →
    Wf sh x → ∀ k, k ≤ Tc → CorrTick.Le sh (srLegs sh x) k (rrLegs sh x)
  | .sing _, x, h, k, hk => x.cpl h k hk
  | .stream _ _ _ _, x, h, k, hk => x.cpl h k hk
  | .ksing _ _ _ _, x, h, k, hk => x.cpl h k hk
  | .pair a b, x, h, k, hk => ⟨cpl a x.1 h.1 k hk, cpl b x.2 h.2 k hk⟩

/-- The machine legs of a well-formed shaped wire are step-monotone,
leafwise. -/
theorem mono {ℓ : L} : (sh : TickShape) → (x : AtomicOf (Tk Tc mem) (TS Tc mem) (TKS Tc mem) ℓ sh) →
    Wf sh x → TMonoOf sh (srLegs sh x)
  | .sing _, x, h => x.mono h
  | .stream _ _ _ _, x, h => x.mono h
  | .ksing _ _ _ _, x, h => x.mono h
  | .pair a b, x, h => ⟨mono a x.1 h.1, mono b x.2 h.2⟩

def mk {ℓ : L} : (sh : TickShape) →
    (m : AtomicOf (SchedTick.Tk mem) (SchedTick.TS mem) (SchedTick.TKS mem) ℓ sh) →
    (v : AtomicOf (ValuesTick.Tk mem) (ValuesTick.TS mem) (ValuesTick.TKS mem) ℓ sh) →
    (wf : Prop) → (wf → ∀ k, k ≤ Tc → CorrTick.Le sh m k v) → (wf → TMonoOf sh m) →
    AtomicOf (Tk Tc mem) (TS Tc mem) (TKS Tc mem) ℓ sh
  | .sing _, m, v, wf, h, hm => ⟨m, v, wf, h, hm⟩
  | .stream _ _ _ _, m, v, wf, h, hm => ⟨m, v, wf, h, hm⟩
  | .ksing _ _ _ _, m, v, wf, h, hm => ⟨m, v, wf, h, hm⟩
  | .pair a b, m, v, wf, h, hm =>
    (mk a m.1 v.1 wf (fun hw k hk => (h hw k hk).1) (fun hw => (hm hw).1),
     mk b m.2 v.2 wf (fun hw k hk => (h hw k hk).2) (fun hw => (hm hw).2))

/-! ### Bounded tuples: projections, embeddings, coercion -/

def projSR : (sh : TickShape) → BoundedOf CoBSing CoBounded CoBKSing sh →
    BoundedOf SchedTick.BS SchedTick.BStr SchedTick.BKS sh
  | .sing _, b => b.sr
  | .stream _ _ _ _, b => b.sr
  | .ksing _ _ _ _, b => b.sr
  | .pair a c, b => (projSR a b.1, projSR c b.2)

def projRR : (sh : TickShape) → BoundedOf CoBSing CoBounded CoBKSing sh →
    BoundedOf ValuesTick.BS PoolCarrier ValuesTick.BKS sh
  | .sing _, b => b.rr
  | .stream _ _ _ _, b => b.rr
  | .ksing _ _ _ _, b => b.rr
  | .pair a c, b => (projRR a b.1, projRR c b.2)

def WfB : (sh : TickShape) → BoundedOf CoBSing CoBounded CoBKSing sh → Prop
  | .sing _, b => b.wf
  | .stream _ _ _ _, b => b.wf
  | .ksing _ _ _ _, b => b.wf
  | .pair a c, b => WfB a b.1 ∧ WfB c b.2

/-- The machine leg alone (the denotation leg dummy, uncoupled). -/
def embedSR : (sh : TickShape) → BoundedOf SchedTick.BS SchedTick.BStr SchedTick.BKS sh →
    BoundedOf CoBSing CoBounded CoBKSing sh
  | .sing _, v => ⟨v, v, False, fun h => h.elim⟩
  | .stream _ _ ord ret, l => ⟨l, PoolBot ord ret, False, fun h => h.elim⟩
  | .ksing _ _ _ _, m => ⟨m, KeyedMap.empty, False, fun h => h.elim⟩
  | .pair a c, b => (embedSR a b.1, embedSR c b.2)

/-- The denotation leg alone (the machine leg dummy, uncoupled). -/
def embedRR : (sh : TickShape) → BoundedOf ValuesTick.BS PoolCarrier ValuesTick.BKS sh →
    BoundedOf CoBSing CoBounded CoBKSing sh
  | .sing _, v => ⟨v, v, False, fun h => h.elim⟩
  | .stream _ _ _ _, p => ⟨[], p, False, fun h => h.elim⟩
  | .ksing _ _ _ _, m => ⟨SchedKMap.empty, m, False, fun h => h.elim⟩
  | .pair a c, b => (embedRR a b.1, embedRR c b.2)

/-- Both legs, `wf` exactly when coupled. -/
def embed2 : (sh : TickShape) → BoundedOf SchedTick.BS SchedTick.BStr SchedTick.BKS sh →
    BoundedOf ValuesTick.BS PoolCarrier ValuesTick.BKS sh → BoundedOf CoBSing CoBounded CoBKSing sh
  | .sing _, m, v => ⟨m, v, m = v, id⟩
  | .stream _ _ ord ret, m, v => ⟨m, v, CoupledBatch ord ret m v, id⟩
  | .ksing _ _ _ _, m, v => ⟨m, v, KeyedMap.ofAssoc m = v, id⟩
  | .pair a c, m, v => (embed2 a m.1 v.1, embed2 c m.2 v.2)

/-- The machine tuple as the denotation's. -/
def coerce : (sh : TickShape) → BoundedOf SchedTick.BS SchedTick.BStr SchedTick.BKS sh →
    BoundedOf ValuesTick.BS PoolCarrier ValuesTick.BKS sh
  | .sing _, v => v
  | .stream _ _ ord ret, l => poolOfListR ord ret l
  | .ksing _ _ _ _, m => KeyedMap.ofAssoc m
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
    (b : BoundedOf CoBSing CoBounded CoBKSing sh) → WfB sh b →
    coerce sh (projSR sh b) = projRR sh b
  | .sing _, _, b, h => b.cpl h
  | .stream _ _ ord ret, heo, b, h => by
    obtain rfl := allEO_stream heo
    have hc := b.cpl h
    cases ord
    · exact hc
    · exact hc
  | .ksing _ _ _ _, _, b, h => b.cpl h
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
    (m : AtomicOf (SchedTick.Tk mem) (SchedTick.TS mem) (SchedTick.TKS mem) ℓ sh) → (k : Nat) →
    (v : AtomicOf (ValuesTick.Tk mem) (ValuesTick.TS mem) (ValuesTick.TKS mem) ℓ sh) →
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
  | .ksing _ _ _ _, _, m, k, v, h, i => h i
  | .pair a c, heo, m, k, v, h, i => by
    show (Trace.zip (SchedTick.slice a m.1 i k) (SchedTick.slice c m.2 i k)).map
        (fun p => (coerce a p.1, coerce c p.2))
      <+: Trace.zip (ValuesTick.slice a v.1 i) (ValuesTick.slice c v.2 i)
    rw [← zip_map_both]
    exact zip_prefix (slice_le a (allEO_pair heo).1 m.1 k v.1 h.1 i)
      (slice_le c (allEO_pair heo).2 m.2 k v.2 h.2 i)

theorem map_coerce_sing (τ : Type) :
    ∀ (l : List (BoundedOf SchedTick.BS SchedTick.BStr SchedTick.BKS (.sing τ))),
      l.map (coerce (.sing τ)) = l
  | [] => rfl
  | _ :: xs => congrArg (List.cons _) (map_coerce_sing τ xs)

theorem map_coerce_total (α : Type) (inst : DecidableEq α) :
    ∀ (l : List (BoundedOf SchedTick.BS SchedTick.BStr SchedTick.BKS
        (.stream α inst .totalOrder .exactlyOnce))),
      l.map (coerce (.stream α inst .totalOrder .exactlyOnce)) = l
  | [] => rfl
  | _ :: xs => congrArg (List.cons _) (map_coerce_total α inst xs)

theorem map_fst_map_coerce (a c : TickShape) :
    ∀ (l : List (BoundedOf SchedTick.BS SchedTick.BStr SchedTick.BKS (.pair a c))),
      (l.map (coerce (.pair a c))).map Prod.fst = (l.map Prod.fst).map (coerce a)
  | [] => rfl
  | _ :: xs => congrArg (List.cons _) (map_fst_map_coerce a c xs)

theorem map_snd_map_coerce (a c : TickShape) :
    ∀ (l : List (BoundedOf SchedTick.BS SchedTick.BStr SchedTick.BKS (.pair a c))),
      (l.map (coerce (.pair a c))).map Prod.snd = (l.map Prod.snd).map (coerce c)
  | [] => rfl
  | _ :: xs => congrArg (List.cons _) (map_snd_map_coerce a c xs)

/-- Back from coerced output-tuple prefixes to the leafwise coupling. -/
theorem unslice_le {ℓ : L} : (sh : TickShape) → allEO sh = true →
    (F : Fin (mem ℓ) → Nat → Trace (BoundedOf SchedTick.BS SchedTick.BStr SchedTick.BKS sh)) →
    (k : Nat) → (G : MemberMap (mem ℓ) (Trace (BoundedOf ValuesTick.BS PoolCarrier ValuesTick.BKS sh))) →
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
  | .ksing _ _ _ _, _, F, k, G, h => fun i => h i
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
  | .ksing K V _ _, _ => by
    show KeyedMap.ofAssoc (SchedKMap.empty : SchedKMap K V) = KeyedMap.empty
    exact KeyedMap.ext rfl rfl
  | .pair a b, s => by
    show (coerce a (SchedTick.seed a s.1), coerce b (SchedTick.seed b s.2)) = _
    rw [coerce_seed a s.1, coerce_seed b s.2]
    rfl

/-! ### The body's legs (the register is a shape like the inputs) -/

section Body
variable {ℓ : L} (sts ins outs : TickShape)
  (g : Fin (mem ℓ) → BoundedOf CoBSing CoBounded CoBKSing sts → BoundedOf CoBSing CoBounded CoBKSing ins →
    BoundedOf CoBSing CoBounded CoBKSing sts × BoundedOf CoBSing CoBounded CoBKSing outs)

/-- The machine body: the body on the machine leg alone. -/
def gS (i : Fin (mem ℓ)) (s : BoundedOf SchedTick.BS SchedTick.BStr SchedTick.BKS sts)
    (inp : BoundedOf SchedTick.BS SchedTick.BStr SchedTick.BKS ins) :
    BoundedOf SchedTick.BS SchedTick.BStr SchedTick.BKS sts × BoundedOf SchedTick.BS SchedTick.BStr SchedTick.BKS outs :=
  let r := g i (embedSR sts s) (embedSR ins inp)
  (projSR sts r.1, projSR outs r.2)

/-- The denotation body: the body on the denotation leg alone. -/
def gR (i : Fin (mem ℓ)) (s : BoundedOf ValuesTick.BS PoolCarrier ValuesTick.BKS sts)
    (inp : BoundedOf ValuesTick.BS PoolCarrier ValuesTick.BKS ins) :
    BoundedOf ValuesTick.BS PoolCarrier ValuesTick.BKS sts × BoundedOf ValuesTick.BS PoolCarrier ValuesTick.BKS outs :=
  let r := g i (embedRR sts s) (embedRR ins inp)
  (projRR sts r.1, projRR outs r.2)

/-- The coupled run: the body on a machine register and tuple coupled
with their own coercions. -/
def gC (i : Fin (mem ℓ)) (s : BoundedOf SchedTick.BS SchedTick.BStr SchedTick.BKS sts)
    (inp : BoundedOf SchedTick.BS SchedTick.BStr SchedTick.BKS ins) :
    BoundedOf CoBSing CoBounded CoBKSing sts × BoundedOf CoBSing CoBounded CoBKSing outs :=
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
    (h3 : W3 sts ins outs g) (i : Fin (mem ℓ)) (s : BoundedOf SchedTick.BS SchedTick.BStr SchedTick.BKS sts)
    (inp : BoundedOf SchedTick.BS SchedTick.BStr SchedTick.BKS ins) :
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
    ∀ (l : Trace (BoundedOf SchedTick.BS SchedTick.BStr SchedTick.BKS ins))
      (s : BoundedOf SchedTick.BS SchedTick.BStr SchedTick.BKS sts),
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
    (x : AtomicOf (Tk Tc mem) (TS Tc mem) (TKS Tc mem) ℓ ins)
    (g : Fin (mem ℓ) → BoundedOf CoBSing CoBounded CoBKSing sts → BoundedOf CoBSing CoBounded CoBKSing ins →
      BoundedOf CoBSing CoBounded CoBKSing sts × BoundedOf CoBSing CoBounded CoBKSing outs) : Prop :=
  Wf ins x ∧ allEO sts = true ∧ allEO ins = true ∧ allEO outs = true
    ∧ W1 sts ins outs g ∧ W2 sts ins outs g ∧ W3 sts ins outs g

/-- The coupling of the two legs' tick formers, under `scanWf`. -/
theorem scan_cpl (pacing : Pacing L mem) {ℓ : L}
    (sts ins outs : TickShape) (x : AtomicOf (Tk Tc mem) (TS Tc mem) (TKS Tc mem) ℓ ins)
    (g : Fin (mem ℓ) → BoundedOf CoBSing CoBounded CoBKSing sts → BoundedOf CoBSing CoBounded CoBKSing ins →
      BoundedOf CoBSing CoBounded CoBKSing sts × BoundedOf CoBSing CoBounded CoBKSing outs)
    (init : SeedOf sts) :
    scanWf sts ins outs x g → ∀ k, k ≤ Tc →
      CorrTick.Le outs
        ((SchedSem L mem pacing).tick_scan sts ins outs (srLegs ins x) (gS sts ins outs g) init) k
        ((Values L mem).tick_scan sts ins outs (rrLegs ins x) (gR sts ins outs g) init) :=
  fun ⟨hx, heoS, heoI, heoO, h1, h2, h3⟩ k hk =>
    unslice_le outs heoO
      (fun i k => scanAcrossTicksTrace (gS sts ins outs g i) (SchedTick.seed sts init)
        (SchedTick.slice ins (srLegs ins x) i k)) k
      (.mk' fun i => scanAcrossTicksTrace (gR sts ins outs g i) (ValuesTick.seed sts init)
        (ValuesTick.slice ins (rrLegs ins x) i))
      (fun i => by
        rw [scan_comm sts ins outs g heoS heoO h1 h2 h3 i, coerce_seed]
        exact scanAcrossTicksTrace_prefix _ (ValuesTick.seed sts init)
          (slice_le ins heoI _ k _ (cpl ins x hx k hk) i))

/-- The tick former at the corner: the machine's former on the `sr`
legs, the denotation's on the `rr` legs, coupled under `scanWf`. -/
def scan (pacing : Pacing L mem) {ℓ : L}
    (sts ins outs : TickShape) (x : AtomicOf (Tk Tc mem) (TS Tc mem) (TKS Tc mem) ℓ ins)
    (g : Fin (mem ℓ) → BoundedOf CoBSing CoBounded CoBKSing sts → BoundedOf CoBSing CoBounded CoBKSing ins →
      BoundedOf CoBSing CoBounded CoBKSing sts × BoundedOf CoBSing CoBounded CoBKSing outs)
    (init : SeedOf sts) :
    AtomicOf (Tk Tc mem) (TS Tc mem) (TKS Tc mem) ℓ outs :=
  mk outs ((SchedSem L mem pacing).tick_scan sts ins outs (srLegs ins x) (gS sts ins outs g) init)
    ((Values L mem).tick_scan sts ins outs (rrLegs ins x) (gR sts ins outs g) init)
    (scanWf sts ins outs x g) (scan_cpl pacing sts ins outs x g init)
    (fun hw => tmono_tick_scan sts ins outs (gS sts ins outs g) init (mono ins x hw.1))

/-! ### The structural simp rules (the `co_simp` / `co_wf_simp` walk
over shaped tick formers: leg projections of `mk`, the leaf/pair
equations of every shape-recursive helper) -/

section Rules
variable {ℓ : L}

theorem srLegs_pair (a b : TickShape) (x : AtomicOf (Tk Tc mem) (TS Tc mem) (TKS Tc mem) ℓ (.pair a b)) :
    srLegs (.pair a b) x = (srLegs a x.1, srLegs b x.2) := rfl
theorem srLegs_sing (τ : Type) (x : CoTickSing Tc (mem ℓ) τ) :
    srLegs (mem := mem) (ℓ := ℓ) (.sing τ) x = x.sr := rfl
theorem srLegs_stream (α : Type) (inst : DecidableEq α) (ord : StrOrd)
    (ret : Retries) (x : CoAtomicStream Tc (mem ℓ) α ord ret) :
    srLegs (mem := mem) (ℓ := ℓ) (.stream α inst ord ret) x = x.sr := rfl
theorem rrLegs_pair (a b : TickShape) (x : AtomicOf (Tk Tc mem) (TS Tc mem) (TKS Tc mem) ℓ (.pair a b)) :
    rrLegs (.pair a b) x = (rrLegs a x.1, rrLegs b x.2) := rfl
theorem rrLegs_sing (τ : Type) (x : CoTickSing Tc (mem ℓ) τ) :
    rrLegs (mem := mem) (ℓ := ℓ) (.sing τ) x = x.rr := rfl
theorem rrLegs_stream (α : Type) (inst : DecidableEq α) (ord : StrOrd)
    (ret : Retries) (x : CoAtomicStream Tc (mem ℓ) α ord ret) :
    rrLegs (mem := mem) (ℓ := ℓ) (.stream α inst ord ret) x = x.rr := rfl
theorem Wf_pair (a b : TickShape) (x : AtomicOf (Tk Tc mem) (TS Tc mem) (TKS Tc mem) ℓ (.pair a b)) :
    Wf (.pair a b) x = (Wf a x.1 ∧ Wf b x.2) := rfl
theorem Wf_sing (τ : Type) (x : CoTickSing Tc (mem ℓ) τ) :
    Wf (mem := mem) (ℓ := ℓ) (.sing τ) x = x.wf := rfl
theorem Wf_stream (α : Type) (inst : DecidableEq α) (ord : StrOrd)
    (ret : Retries) (x : CoAtomicStream Tc (mem ℓ) α ord ret) :
    Wf (mem := mem) (ℓ := ℓ) (.stream α inst ord ret) x = x.wf := rfl

theorem projSR_pair (a b : TickShape) (x : BoundedOf CoBSing CoBounded CoBKSing (.pair a b)) :
    projSR (.pair a b) x = (projSR a x.1, projSR b x.2) := rfl
theorem projSR_sing (τ : Type) (x : CoBSing τ) : projSR (.sing τ) x = x.sr := rfl
theorem projSR_stream (α : Type) (inst : DecidableEq α) (ord : StrOrd)
    (ret : Retries) (x : CoBounded α ord ret) :
    projSR (.stream α inst ord ret) x = x.sr := rfl
theorem projRR_pair (a b : TickShape) (x : BoundedOf CoBSing CoBounded CoBKSing (.pair a b)) :
    projRR (.pair a b) x = (projRR a x.1, projRR b x.2) := rfl
theorem projRR_sing (τ : Type) (x : CoBSing τ) : projRR (.sing τ) x = x.rr := rfl
theorem projRR_stream (α : Type) (inst : DecidableEq α) (ord : StrOrd)
    (ret : Retries) (x : CoBounded α ord ret) :
    projRR (.stream α inst ord ret) x = x.rr := rfl
theorem WfB_pair (a b : TickShape) (x : BoundedOf CoBSing CoBounded CoBKSing (.pair a b)) :
    WfB (.pair a b) x = (WfB a x.1 ∧ WfB b x.2) := rfl
theorem WfB_sing (τ : Type) (x : CoBSing τ) : WfB (.sing τ) x = x.wf := rfl
theorem WfB_stream (α : Type) (inst : DecidableEq α) (ord : StrOrd)
    (ret : Retries) (x : CoBounded α ord ret) :
    WfB (.stream α inst ord ret) x = x.wf := rfl

theorem embedSR_pair (a b : TickShape)
    (x : BoundedOf SchedTick.BS SchedTick.BStr SchedTick.BKS (.pair a b)) :
    embedSR (.pair a b) x = (embedSR a x.1, embedSR b x.2) := rfl
theorem embedSR_sing_sr (τ : Type) (v : τ) : (embedSR (.sing τ) v).sr = v := rfl
theorem embedSR_sing_rr (τ : Type) (v : τ) : (embedSR (.sing τ) v).rr = v := rfl
theorem embedSR_sing_wf (τ : Type) (v : τ) : (embedSR (.sing τ) v).wf = False := rfl
theorem embedSR_stream_sr (α : Type) (inst : DecidableEq α) (ord : StrOrd)
    (ret : Retries) (l : List α) : (embedSR (.stream α inst ord ret) l).sr = l := rfl
theorem embedSR_stream_wf (α : Type) (inst : DecidableEq α) (ord : StrOrd)
    (ret : Retries) (l : List α) : (embedSR (.stream α inst ord ret) l).wf = False := rfl
theorem embedRR_pair (a b : TickShape)
    (x : BoundedOf ValuesTick.BS PoolCarrier ValuesTick.BKS (.pair a b)) :
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
    (m : BoundedOf SchedTick.BS SchedTick.BStr SchedTick.BKS (.pair a b))
    (v : BoundedOf ValuesTick.BS PoolCarrier ValuesTick.BKS (.pair a b)) :
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
    (m : BoundedOf SchedTick.BS SchedTick.BStr SchedTick.BKS (.pair a b)) :
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

/-- The `.ksing` leaf rules. -/
theorem srLegs_ksing (K V : Type) (iK : DecidableEq K) (iV : DecidableEq V)
    (x : CoAtomicKSing Tc (mem ℓ) K V) :
    srLegs (mem := mem) (ℓ := ℓ) (.ksing K V iK iV) x = x.sr := rfl
theorem rrLegs_ksing (K V : Type) (iK : DecidableEq K) (iV : DecidableEq V)
    (x : CoAtomicKSing Tc (mem ℓ) K V) :
    rrLegs (mem := mem) (ℓ := ℓ) (.ksing K V iK iV) x = x.rr := rfl
theorem Wf_ksing (K V : Type) (iK : DecidableEq K) (iV : DecidableEq V)
    (x : CoAtomicKSing Tc (mem ℓ) K V) :
    Wf (mem := mem) (ℓ := ℓ) (.ksing K V iK iV) x = x.wf := rfl
theorem projSR_ksing (K V : Type) (iK : DecidableEq K) (iV : DecidableEq V)
    (x : CoBKSing K V) : projSR (.ksing K V iK iV) x = x.sr := rfl
theorem projRR_ksing (K V : Type) (iK : DecidableEq K) (iV : DecidableEq V)
    (x : CoBKSing K V) : projRR (.ksing K V iK iV) x = x.rr := rfl
theorem WfB_ksing (K V : Type) (iK : DecidableEq K) (iV : DecidableEq V)
    (x : CoBKSing K V) : WfB (.ksing K V iK iV) x = x.wf := rfl
theorem embedSR_ksing_sr (K V : Type) (iK : DecidableEq K) (iV : DecidableEq V)
    (m : SchedKMap K V) : (embedSR (.ksing K V iK iV) m).sr = m := rfl
theorem embedSR_ksing_wf (K V : Type) (iK : DecidableEq K) (iV : DecidableEq V)
    (m : SchedKMap K V) : (embedSR (.ksing K V iK iV) m).wf = False := rfl
theorem embedRR_ksing_rr (K V : Type) (iK : DecidableEq K) (iV : DecidableEq V)
    (v : KeyedMap K V) : (embedRR (.ksing K V iK iV) v).rr = v := rfl
theorem embedRR_ksing_wf (K V : Type) (iK : DecidableEq K) (iV : DecidableEq V)
    (v : KeyedMap K V) : (embedRR (.ksing K V iK iV) v).wf = False := rfl
theorem embed2_ksing_sr (K V : Type) (iK : DecidableEq K) (iV : DecidableEq V)
    (m : SchedKMap K V) (v : KeyedMap K V) : (embed2 (.ksing K V iK iV) m v).sr = m := rfl
theorem embed2_ksing_rr (K V : Type) (iK : DecidableEq K) (iV : DecidableEq V)
    (m : SchedKMap K V) (v : KeyedMap K V) : (embed2 (.ksing K V iK iV) m v).rr = v := rfl
theorem embed2_ksing_wf (K V : Type) (iK : DecidableEq K) (iV : DecidableEq V)
    (m : SchedKMap K V) (v : KeyedMap K V) :
    (embed2 (.ksing K V iK iV) m v).wf = (KeyedMap.ofAssoc m = v) := rfl
theorem coerce_ksing (K V : Type) (iK : DecidableEq K) (iV : DecidableEq V)
    (m : SchedKMap K V) : coerce (.ksing K V iK iV) m = KeyedMap.ofAssoc m := rfl
theorem allEO_ksing (K V : Type) (iK : DecidableEq K) (iV : DecidableEq V) :
    allEO (.ksing K V iK iV) = true := rfl

theorem allEO_sing (τ : Type) : allEO (.sing τ) = true := rfl
theorem allEO_stream_eo (α : Type) (inst : DecidableEq α) (ord : StrOrd) :
    allEO (.stream α inst ord .exactlyOnce) = true := rfl
theorem allEO_pair_eq (a b : TickShape) :
    allEO (.pair a b) = (allEO a && allEO b) := rfl

theorem gS_def (sts ins outs : TickShape)
    (g : Fin (mem ℓ) → BoundedOf CoBSing CoBounded CoBKSing sts → BoundedOf CoBSing CoBounded CoBKSing ins →
      BoundedOf CoBSing CoBounded CoBKSing sts × BoundedOf CoBSing CoBounded CoBKSing outs) :
    gS sts ins outs g = fun i s inp =>
      (projSR sts (g i (embedSR sts s) (embedSR ins inp)).1,
       projSR outs (g i (embedSR sts s) (embedSR ins inp)).2) := rfl
theorem gR_def (sts ins outs : TickShape)
    (g : Fin (mem ℓ) → BoundedOf CoBSing CoBounded CoBKSing sts → BoundedOf CoBSing CoBounded CoBKSing ins →
      BoundedOf CoBSing CoBounded CoBKSing sts × BoundedOf CoBSing CoBounded CoBKSing outs) :
    gR sts ins outs g = fun i s inp =>
      (projRR sts (g i (embedRR sts s) (embedRR ins inp)).1,
       projRR outs (g i (embedRR sts s) (embedRR ins inp)).2) := rfl
theorem gC_def (sts ins outs : TickShape)
    (g : Fin (mem ℓ) → BoundedOf CoBSing CoBounded CoBKSing sts → BoundedOf CoBSing CoBounded CoBKSing ins →
      BoundedOf CoBSing CoBounded CoBKSing sts × BoundedOf CoBSing CoBounded CoBKSing outs) :
    gC sts ins outs g = fun i s inp =>
      g i (embed2 sts s (coerce sts s)) (embed2 ins inp (coerce ins inp)) := rfl

end Rules

end CoTick

/-- `get_max_key` transports a keyed singleton's read agreement to the
optional's: the machine's association-list maximum is the map's. -/
theorem CoKSing.maxEntry_cpl {T n : Nat} {α K V : Type} [DecidableEq α] [DecidableEq K]
    [DecidableEq V] [LinearOrder K] {ord : StrOrd} {ret : Retries}
    (ks : CoKSing T n α K V ord ret .boundedValue) (hwf : ks.wf) :
    ∃ (rd : Fin n → List α → Option (K × V)),
      (∀ i t, (ks.sr i t).maxEntry = rd i ((ks.src i).view t)) ∧
      (∀ i (ws : List (List α)), List.IsChain (· <+: ·) ([] :: ws) →
        (∀ w ∈ ws, w <+: (ks.src i).view T) →
        ws.map (rd i) = (ks.rr i (snapCut .noOrder ret ws)).map KeyedMap.maxEntry) ∧
      (∀ i d d', CutLe .noOrder d d' →
        (ks.rr i d).map KeyedMap.maxEntry <+: (ks.rr i d').map KeyedMap.maxEntry) := by
  obtain ⟨rd, hval, hagree, hmono⟩ := ks.cpl hwf
  refine ⟨fun i w => (rd i w).maxEntry, fun i t => by rw [hval i t], ?_, ?_⟩
  · intro i ws hchain hpre
    have h := hagree i ws hchain hpre
    have hcut : snapCut .noOrder ret ws = cutsMS (0 : Multiset α) ws := by cases ret <;> rfl
    have h2 := List.Forall₂.map_eq (f := fun l : SchedKMap K V => l.maxEntry)
      (g := KeyedMap.maxEntry) (fun l m hlm => by rw [← hlm, KeyedMap.maxEntry_ofAssoc]) h
    rw [hcut, ← h2, List.map_map]
    rfl
  · intro i d d' hdd
    exact (hmono i d d' hdd).map _

set_option warn.classDefReducibility false in
/-- Couple at `Tc`, derive at `Td` (`Tc ≤ Td`). Programs run at
`(T, T)`; the knots' graded couplings re-instantiate their generic
bodies at lowered couple horizons `(j, T)`. -/
def CoupleSem (L : Type) (mem : L → Nat)
    (pacing : Pacing L mem)
    (Tc Td : Nat) (hjT : Tc ≤ Td) : HydroSem L mem where
  Stream ℓ α _ ord ret := CoStream Tc (mem ℓ) α ord ret
  KeyedStream ℓ K V _ _ ord ret := CoKStream Tc (mem ℓ) K V ord ret
  Singleton ℓ α σ _ ord ret b := CoSing Tc (mem ℓ) α σ ord ret b
  KeyedSingleton ℓ α K V _ _ _ ord ret b := CoKSing Tc (mem ℓ) α K V ord ret b
  Atomic ℓ σ := CoTickSing Tc (mem ℓ) σ
  -- one tick's content: both representations with their coupling
  BoundedStream α _ ord ret := CoBounded α ord ret
  BoundedSingleton σ := CoBSing σ
  BoundedKeyedStream K V _ _ ord ret := CoBKStream K V ord ret
  BoundedKeyedSingleton K V _ _ := CoBKSing K V
  -- the keyed-singleton wire: two legs with different history axes (as
  -- `AtomicStream`)
  AtomicKeyedSingleton ℓ K V _ _ := CoAtomicKSing Tc (mem ℓ) K V
  AtomicKeyedStream ℓ K V _ _ ord ret := CoAtomicKStream Tc (mem ℓ) K V ord ret
  -- overrides the signature default: the corner couples a machine
  -- tick-stream wire (step-indexed lists) with a denotation wire
  -- (quotient batches) — two legs with different history axes
  AtomicStream ℓ α _ ord ret := CoAtomicStream Tc (mem ℓ) α ord ret
  TransportDec p c := Fin p → Fin c → Nat → Nat
  OrderSelDec _ _ := Unit
  TickOrderSelDec _ _ := Unit
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
  broadcast_closed {_c p _α _ _ord _ret} dt s :=
    { sr := fun i => mergeN (fun j => ((s.sr j).deliver (pacing.active p i) (dt i j)).map (fun v => (j, v)))
      rr := (Values L mem).broadcast_closed () s.rr
      wf := s.wf
      cpl := fun hwf i =>
        corr_transport (fun j => deliver_view_cpl (s.sr j) (pacing.active p i) (dt i j) Tc (s.cpl hwf j)) }
  demux {_c p _α _ _ord} dt s addr :=
    { sr := fun i => mergeN (fun j => (((s.sr j).filterMap
          (fun dx => if dx.1 = addr i then some dx.2 else none)).deliver
          (pacing.active p i) (dt i j)).map (fun v => (j, v)))
      rr := (Values L mem).demux () s.rr addr
      wf := s.wf
      cpl := fun hwf i =>
        corr_transport (fun j =>
          deliver_view_cpl ((s.sr j).filterMap _) (pacing.active p i) (dt i j) Tc
            (by
              refine ListLe.of_prefix (List.prefix_refl _) ?_
              exact ListLe.filterMap_eo _ (s.cpl hwf j))) }
  values k :=
    { sr := fun i => (k.sr i).map Prod.snd
      rr := (Values L mem).values k.rr
      wf := k.wf
      cpl := fun hwf i => corr_values (k.cpl hwf i) }
  into_keyed s :=
    { sr := s.sr
      rr := (Values L mem).into_keyed s.rr
      wf := s.wf
      cpl := fun hwf i => corr_into_keyed (s.cpl hwf i) }
  kassume_ordering u _sel :=
    { sr := u.sr
      rr := (Values L mem).kassume_ordering u.rr (ordSelDerive Td u.sr)
      wf := u.wf
      cpl := fun hwf i => by
        show KeyedListLe .totalOrder .exactlyOnce ((u.sr i).view Tc)
          (KeyedPool.ofList (selectOrder (u.rr i).toMultiset (ordSelDerive Td u.sr i)))
        obtain ⟨e, he⟩ := ordSelDerive_mono hjT u.sr i
        have hpre : (u.sr i).view Tc <+: selectOrder (u.rr i).toMultiset (ordSelDerive Td u.sr i) := by
          rw [← he]
          exact selectOrder_prefix_ext _ e _ (u.cpl hwf i).coe_le_toMultiset
        exact fun k => keyVals_prefix hpre }
  kfold_early_stop {_ℓ K V _A _ _ _} ks f init :=
    { sr := fun i t => keyedFoldEarlyStopAssoc f init ((ks.sr i).view t)
      rr := (Values L mem).kfold_early_stop ks.rr f init
      src := ks.sr
      wf := ks.wf
      cpl := fun hwf => by
        refine ⟨fun _i w => keyedFoldEarlyStopAssoc f init w, fun _i _t => rfl, ?_, ?_⟩
        · intro i ws hchain hpre
          show List.Forall₂ (fun l m => KeyedMap.ofAssoc l = m)
            (ws.map (keyedFoldEarlyStopAssoc f init))
            (((ks.rr i).kviews 0 (cutsMS 0 ws)).map
              (fun M => KeyedMap.ofFoldEarlyStop f init ((ks.rr i).readAt M)))
          have hpre' : ∀ w ∈ ws, ∀ k, keyVals k w <+: (ks.rr i).grp k :=
            fun w hw k => (keyVals_prefix (hpre w hw)).trans (ks.cpl hwf i k)
          rw [show (0 : Multiset (K × V)) = Multiset.ofList ([] : List (K × V)) from rfl,
            kviews_cutsMS (ks.rr i) ws [] hchain
              (fun w hw => KeyedMap.legalView_of_keyed_prefix (hpre' w hw)),
            List.map_map]
          refine List.forall₂_map_left_iff.mpr (List.forall₂_map_right_iff.mpr
            (List.forall₂_same.mpr fun w hw => ?_))
          show KeyedMap.ofAssoc (keyedFoldEarlyStopAssoc f init w)
            = KeyedMap.ofFoldEarlyStop f init ((ks.rr i).readAt ↑w)
          rw [KeyedMap.ofAssoc_keyedFoldEarlyStopAssoc, KeyedMap.readAt_of_keyed_prefix (hpre' w hw)]
        · intro i d d' hdd
          exact (KeyedPool.kviews_mono_dec _ _ hdd).map _ }
  kget_max_key {_ℓ _α _K _V _ _ _ _ ord ret} ks :=
    { sr := fun i t => (ks.sr i t).maxEntry
      rr := (Values L mem).kget_max_key (ord := ord) (ret := ret) ks.rr
      src := ks.src
      wf := ks.wf
      cpl := fun hwf => CoKSing.maxEntry_cpl ks hwf }
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
  -- the conditional fold: the corner's `wf` carries the pairwise content
  -- fact over the denotation leg (what the program's `ghost pre` proves)
  fold {_ℓ _α _σ _ ord ret} g init Pre _ ok s :=
    { sr := fun i => schedFold g init (s.sr i)
      rr := (Values L mem).fold g init Pre ok s.rr
      src := s.sr
      wf := s.wf ∧ ∀ i, PoolPairwise ord ret Pre (s.rr i)
      cpl := fun hwf =>
        ⟨fun _i l => l.foldl g init, fun _i _t => rfl,
         fun i ws hchain hpre => sched_snap_eq' ord ret Pre g init ok (s.rr i) ws hchain
           (fun w hw => ListLe.of_prefix (hpre w hw) (s.cpl hwf.1 i)) (hwf.2 i),
         fun i _d _d' h => snapTrace_mono_dec Pre g init ok (s.rr i) h⟩ }
  fold_monotone {_ℓ _α _σ _ ord ret} vo g init ok hinfl s :=
    { sr := fun i => schedFold g init (s.sr i)
      rr := (Values L mem).fold_monotone vo g init ok hinfl s.rr
      src := s.sr
      wf := s.wf
      cpl := fun hwf =>
        ⟨fun _i l => l.foldl g init, fun _i _t => rfl,
         fun i ws hchain hpre => sched_snap_eq' ord ret _ g init ok (s.rr i) ws hchain
           (fun w hw => ListLe.of_prefix (hpre w hw) (s.cpl hwf i)) (PoolPairwise_true _ _ _),
         fun i _d _d' h => snapTrace_mono_dec _ g init ok (s.rr i) h⟩ }
  snapshot {ℓ _α _σ _ ord ret b} s _cutd :=
    { sr := fun i t => (tickSteps (pacing.ticks ℓ i) t).map (fun st => s.sr i st)
      rr := (Values L mem).snapshot (ret := ret) s.rr
        (snapDerive ord ret (pacing.ticks ℓ) Td s.src)
      wf := s.wf
      cpl := fun hwf k hk i => by
        obtain ⟨rd, hval, hagree, hmono⟩ := s.cpl hwf
        have hws : (tickSteps (pacing.ticks ℓ i) k).map (fun st => s.sr i st)
            = ((tickSteps (pacing.ticks ℓ i) k).map (fun st => (s.src i).view st)).map (rd i) := by
          rw [List.map_map]
          exact List.map_congr_left (fun st _ => hval i st)
        have heq := hagree i ((tickSteps (pacing.ticks ℓ i) k).map (fun st => (s.src i).view st))
          (viewsChain _ (tickSteps_pairwise (pacing.ticks ℓ i) k))
          (fun w hw => by
            obtain ⟨st, hst, rfl⟩ := List.mem_map.mp hw
            exact (s.src i).mono_le ((tickSteps_le hst).trans hk))
        have hext := hmono i _ _
          (snapDerive_mono ord ret (pacing.ticks ℓ) (hk.trans hjT) s.src i)
        show _ <+: ((Values L mem).snapshot (ℓ := ℓ)
          (ord := ord) (ret := ret) s.rr
          (snapDerive ord ret (pacing.ticks ℓ) Td s.src)) i
        rw [tickVals_snapshot, hws, heq]
        exact hext
      mono := fun _ i _k _k' hk => List.IsPrefix.map _ (tickSteps_le_steps hk) }
  batch {ℓ _α _} s _cutd :=
    { sr := fun i t => batchesFrom ((s.sr i).view)
        (tickSteps (pacing.ticks ℓ i) t) 0
      rr := (Values L mem).batch s.rr (batchDerive (pacing.ticks ℓ) Td s.sr)
      wf := s.wf
      cpl := fun hwf k hk i => by
        show (batchesFrom ((s.sr i).view) (tickSteps (pacing.ticks ℓ i) k)
            0).map (fun b => Multiset.ofList b)
          <+: batchCuts (s.rr i) 0 (batchDerive (pacing.ticks ℓ) Td s.sr i)
        have hreal : batchCuts (s.rr i) 0
            (batchDerive (pacing.ticks ℓ) k s.sr i)
            = batchDerive (pacing.ticks ℓ) k s.sr i :=
          corr_batch (pacing.ticks ℓ) k s.sr (fun j => s.rr j)
            (fun j => s.cpl_le hwf hk j) i
        have hmono := batchCuts_mono_dec (pool := s.rr i)
          (c := 0) (batchDerive_mono (pacing.ticks ℓ) (hk.trans hjT) s.sr i)
        rw [hreal] at hmono
        exact hmono
      mono := fun _ => tmono_batch (pacing := pacing) s.sr () }
  batch_ordered {ℓ _α _} s _cutd :=
    { sr := fun i t => batchesFrom ((s.sr i).view)
        (tickSteps (pacing.ticks ℓ i) t) 0
      rr := (Values L mem).batch_ordered s.rr
        (batchOrdDerive (pacing.ticks ℓ) Td s.sr)
      wf := s.wf
      cpl := fun hwf k hk i => by
        show batchesFrom ((s.sr i).view) (tickSteps (pacing.ticks ℓ i) k) 0
          <+: sliceCuts (s.rr i) 0 (batchOrdDerive (pacing.ticks ℓ) Td s.sr i)
        have hreal : sliceCuts (s.rr i) 0
            (batchOrdDerive (pacing.ticks ℓ) k s.sr i)
            = batchesFrom ((s.sr i).view) (tickSteps (pacing.ticks ℓ i) k) 0 :=
          corr_batch_ordered (pacing.ticks ℓ) k s.sr (fun j => s.rr j)
            (fun j => s.cpl_le hwf hk j) i
        have hmono := sliceCuts_mono_dec (pool := s.rr i)
          (c := 0) (batchOrdDerive_mono (pacing.ticks ℓ) (hk.trans hjT) s.sr i)
        rw [hreal] at hmono
        exact hmono
      mono := fun _ => tmono_batch_ordered (pacing := pacing) s.sr () }
  mapTick s f :=
    { sr := fun i step => (s.sr i step).map (f i)
      rr := (Values L mem).mapTick s.rr f
      wf := s.wf
      cpl := fun hwf k hk i =>
        List.IsPrefix.map (f i) (s.cpl hwf k hk i)
      mono := fun hwf => tmono_mapTick (pacing := pacing) f (s.mono hwf) }
  zipTick a b :=
    { sr := fun i step => Trace.zip (a.sr i step) (b.sr i step)
      rr := (Values L mem).zipTick a.rr b.rr
      wf := a.wf ∧ b.wf
      cpl := fun hwf k hk i =>
        zip_prefix (a.cpl hwf.1 k hk i) (b.cpl hwf.2 k hk i)
      mono := fun hwf => tmono_zipTick (pacing := pacing) (a.mono hwf.1) (b.mono hwf.2) }
  defer_tick {ℓ _σ} init t :=
    { sr := fun i step =>
        if (tickSteps (pacing.ticks ℓ i) step).isEmpty then [] else init :: t.sr i step
      rr := (Values L mem).defer_tick init t.rr
      wf := t.wf
      cpl := fun hwf k hk i => by
        show (if (tickSteps (pacing.ticks ℓ i) k).isEmpty then [] else init :: t.sr i k) <+: init :: t.rr i
        split
        · exact List.nil_prefix
        · exact List.cons_prefix_cons.mpr ⟨rfl, t.cpl hwf k hk i⟩
      mono := fun hwf => tmono_defer_tick (pacing := pacing) init (t.mono hwf) }
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
        (tickSteps (pacing.ticks ℓ i) step).length
      rr := (Values L mem).timeout_snapshot s.rr verd
      wf := True
      cpl := fun _hwf _k _hk i => List.take_prefix _ _
      mono := fun _ _i _k _k' hk =>
        List.take_prefix_take_left (tickSteps_le_steps hk).length_le }
  source_interval_batch {ℓ} pulses :=
    { sr := fun i step => (pulses i).take
        (tickSteps (pacing.ticks ℓ i) step).length
      rr := (Values L mem).source_interval_batch pulses
      wf := True
      cpl := fun _hwf _k _hk i => List.take_prefix _ _
      mono := fun _ _i _k _k' hk =>
        List.take_prefix_take_left (tickSteps_le_steps hk).length_le }
  flattenOrdered t :=
    { sr := t.sr
      rr := (Values L mem).flattenOrdered t.rr
      wf := t.wf
      cpl := fun hwf k hk i => t.cpl hwf k hk i
      mono := fun hwf => t.mono hwf }
  flattenUnordered t :=
    { sr := t.sr
      rr := (Values L mem).flattenUnordered t.rr
      wf := t.wf
      cpl := fun hwf k hk i =>
        List.IsPrefix.map _ (t.cpl hwf k hk i)
      mono := fun hwf => t.mono hwf }
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
    { sr := CoStream.fixSr (fun j => pacing.active ℓ j) body
      rr := iterate (CoStream.vbody (fun j => pacing.active ℓ j) body)
        (.mk' fun _i => PoolBot ord ret) (Td + 1)
      wf := (∀ (h : Nat) (x y : Fin (mem ℓ) → StepHist α),
              SAgree h x y →
              SAgree h (CoStream.sbody body x) (CoStream.sbody body y))
          ∧ (∀ m i, PoolLe ord ret
              (iterate (CoStream.vbody (fun j => pacing.active ℓ j) body)
                (.mk' fun _ => PoolBot ord ret) m i)
              (iterate (CoStream.vbody (fun j => pacing.active ℓ j) body)
                (.mk' fun _ => PoolBot ord ret) (m + 1) i))
          -- the graded coupling, at the Kleene STAGE `j + 1` (the only
          -- denotation `co_fix_cpl` ever probes — so a body may use what
          -- the stage chain proves of its loop input: D71's knot `requires`)
          ∧ (∀ j, j ≤ Tc → ∀ (x : Fin (mem ℓ) → StepHist α),
              (∀ t, t ≤ j → ∀ i, ListLe ord ret ((x i).view t)
                (iterate (CoStream.vbody (fun j => pacing.active ℓ j) body) (.mk' fun _ => PoolBot ord ret) (j + 1) i)) →
              ∀ i, ListLe ord ret ((CoStream.sbody body x i).view j)
                ((body (CoStream.probe2 x
                  (iterate (CoStream.vbody (fun j => pacing.active ℓ j) body) (.mk' fun _ => PoolBot ord ret) (j + 1)))).rr i))
      cpl := fun hwf i => by
        obtain ⟨hcaus, hchain, hcplj⟩ := hwf
        refine ListLe.trans_pool
          (co_fix_cpl body (fun j => pacing.active ℓ j) hcaus hcplj Tc (Nat.le_refl _) i) ?_
        refine pool_chain_glue (f := fun m =>
          iterate (CoStream.vbody (fun j => pacing.active ℓ j) body)
            (.mk' fun _ => PoolBot ord ret) m i)
          (a := Tc + 1) (b := Td + 1) (by omega) ?_
        intro m _hm _hmb
        exact hchain m i }
  fix_tick {ℓ σ} _df body :=
    { sr := CoTickSing.fixSr (fun j => pacing.active ℓ j) body
      rr := iterate (CoTickSing.vbody (fun j => pacing.active ℓ j) body)
        (.mk' fun _i => ([] : Trace σ)) (Td + 1)
      wf := (∀ (h : Nat) (x y : Fin (mem ℓ) → Nat → Trace σ),
              TAgree h x y →
              TAgree h (CoTickSing.sbody body x)
                (CoTickSing.sbody body y))
          ∧ (∀ m i,
              (iterate (CoTickSing.vbody (fun j => pacing.active ℓ j) body)
                (.mk' fun _ => ([] : Trace σ)) m i)
              <+: (iterate (CoTickSing.vbody (fun j => pacing.active ℓ j) body)
                (.mk' fun _ => ([] : Trace σ)) (m + 1) i))
          ∧ (∀ j, j ≤ Tc → ∀ (x : Fin (mem ℓ) → Nat → Trace σ),
              (∀ t, t ≤ j → ∀ i, x i t
                <+: iterate (CoTickSing.vbody (fun j => pacing.active ℓ j) body) (.mk' fun _ => ([] : Trace σ)) (j + 1) i) →
              TMono x →
              ∀ i, CoTickSing.sbody body x i j
                <+: (body (CoTickSing.probe2 x
                  (iterate (CoTickSing.vbody (fun j => pacing.active ℓ j) body) (.mk' fun _ => ([] : Trace σ)) (j + 1)))).rr i)
          -- W4 (D71): the machine body preserves step-monotonicity, so
          -- the knot's diagonal is step-monotone (`co_tick_fix_mono`)
          ∧ (∀ (x : Fin (mem ℓ) → Nat → Trace σ), TMono x → TMono (CoTickSing.sbody body x))
      cpl := fun hwf k hk i => by
        obtain ⟨hcaus, hchain, hcplj, hmono⟩ := hwf
        refine (co_tick_fix_cpl (fun j => pacing.active ℓ j) body hcaus hchain hcplj hmono k hk i).trans ?_
        refine trace_chain_glue (f := fun m =>
          iterate (CoTickSing.vbody (fun j => pacing.active ℓ j) body)
            (.mk' fun _ => ([] : Trace σ)) m i)
          (a := k + 1) (b := Td + 1) (by omega) ?_
        intro m _hm _hmb
        exact hchain m i
      mono := fun hwf => co_tick_fix_mono (fun j => pacing.active ℓ j) body hwf.1 hwf.2.2.2 }

  tinto_keyed {_ℓ _K _V _ _ ord} x :=
    { sr := x.sr
      rr := (Values L mem).tinto_keyed x.rr
      wf := x.wf
      cpl := fun hwf k hk i => by
        have hc := x.cpl hwf k hk i
        show (x.sr i k).map KeyedPool.ofBatch <+: (x.rr i).map (KeyedPool.ofPool (ord := ord))
        cases ord
        · exact List.IsPrefix.map _ hc
        · have : (x.sr i k).map KeyedPool.ofBatch
              = ((x.sr i k).map (fun b => Multiset.ofList b)).map KeyedPool.ofMultiset := by
            rw [List.map_map]; rfl
          rw [this]
          exact List.IsPrefix.map _ hc
      mono := fun hwf => x.mono hwf }
  -- the per-tick order selection (D71): the denotation's selection IS
  -- the machine's per-tick arrival order as realized at `Td`; a
  -- realized tick never changes (`mono`), so at every coupled step the
  -- machine's tick is the selection's own prefix and draws itself
  tassume_ordering u _sel :=
    { sr := u.sr
      rr := (Values L mem).tassume_ordering u.rr (tkOrdSelDerive Td u.sr)
      wf := u.wf
      cpl := fun hwf k hk i =>
        tassume_cpl (u.cpl hwf k hk i) (u.mono hwf i k Td (hk.trans hjT))
      mono := fun hwf => u.mono hwf }
  -- the conditional reduce: the corner's `wf` carries the pairwise fact
  -- over the denotation leg's ACCUMULATED content (hereditary to every
  -- prefix the machine has realized)
  reduce_watermark {_ℓ _K _V _ _ _ _ ord} ks wm f Pre _ ok :=
    { sr := fun i t => scanAcrossTicksTrace (reduceWatermarkEager f) (SchedKMap.empty, none)
        (Trace.zip (ks.sr i t) (wm.sr i t))
      rr := (Values L mem).reduce_watermark ks.rr wm.rr f Pre ok
      wf := ks.wf ∧ wm.wf ∧ ∀ i, (KeyedPool.accum (ks.rr i)).Pairwise Pre
      cpl := fun hwf k hk i => by
        show (scanAcrossTicksTrace (reduceWatermarkEager f) (SchedKMap.empty, none)
            (Trace.zip (ks.sr i k) (wm.sr i k))).map KeyedMap.ofAssoc
          <+: scanAcrossTicksTrace (reduceWatermarkStep f Pre ok) (KeyedPool.bot, none)
            (Trace.zip (ks.rr i) (wm.rr i))
        -- DFIR's eager fold is the accumulate form, output for output
        -- (`scan_eager_eq_assoc`); the accumulate form is the denotation's
        -- step under the per-tick keyed coupling and the pairwise fact
        rw [scan_eager_eq_assoc f _ _ _ (EagerSim.init f)]
        have hks := ks.cpl hwf.1 k hk i
        rw [reduce_scan_coupled f Pre ok (Trace.zip (ks.sr i k) (wm.sr i k)) [] KeyedPool.bot none
          CoupledKeyed.bot (fun j => by
            -- the machine's flat prefix, grouped, sits below the
            -- denotation's accumulated content (the pairwise fact is
            -- hereditary downwards)
            rw [List.nil_append]
            exact KeyedPool.pairwise_flat_prefix
              (map_fst_zip_take_prefix (ks.sr i k) (wm.sr i k) j) hks (hwf.2.2 i))]
        refine scanAcrossTicksTrace_prefix _ _ ?_
        have hz := CoTick.zip_map_both (KeyedPool.ofBatch (ord := ord)) (fun w : Option _K => w)
          (ks.sr i k) (wm.sr i k)
        rw [List.map_id'] at hz
        rw [← hz]
        exact zip_prefix hks (wm.cpl hwf.2.1 k hk i)
      mono := fun hwf i k k' hk =>
        scanAcrossTicksTrace_prefix _ _ (zip_prefix (ks.mono hwf.1 i k k' hk) (wm.mono hwf.2.1 i k k' hk)) }
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
  bfold {_α _σ _ ord ret} g init Pre _ ok b :=
    { sr := b.sr.foldl g init, rr := (Values L mem).bfold g init Pre ok b.rr
      wf := b.wf ∧ ret = .exactlyOnce ∧ PoolPairwise ord ret Pre b.rr
      cpl := fun h => CoupledBatch.fold g init Pre ok (b.cpl h.1) h.2.1 h.2.2 }
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
  binto_keyed b :=
    { sr := b.sr, rr := (Values L mem).binto_keyed b.rr, wf := b.wf
      cpl := fun h => CoupledBatch.intoKeyed (b.cpl h) }
  bkfold {_K _V _A _ _ _ _ _ord ret} g init Pre _ ok ks :=
    { sr := keyedFoldAssoc g init ks.sr
      rr := (Values L mem).bkfold g init Pre ok ks.rr
      wf := ks.wf ∧ ret = .exactlyOnce ∧ ks.rr.Pairwise Pre
      cpl := fun h => by
        obtain ⟨hk, rfl, hpre⟩ := h
        exact KeyedMap.ofAssoc_keyedFoldAssoc g init Pre ok (ks.cpl hk) hpre }
  bksFilter m p :=
    { sr := m.sr.filter p, rr := (Values L mem).bksFilter m.rr p, wf := m.wf
      cpl := fun h => by rw [KeyedMap.ofAssoc_filter, m.cpl h]; rfl }
  bksMap m f :=
    { sr := m.sr.map f, rr := (Values L mem).bksMap m.rr f, wf := m.wf
      cpl := fun h => by rw [KeyedMap.ofAssoc_map, m.cpl h]; rfl }
  bksKeys m :=
    { sr := m.sr.keys, rr := (Values L mem).bksKeys m.rr, wf := m.wf
      cpl := fun h => CoupledBatch.ksKeys (m.cpl h) }
  bksEntries m :=
    { sr := m.sr.entries, rr := (Values L mem).bksEntries m.rr, wf := m.wf
      cpl := fun h => CoupledBatch.ksEntries (m.cpl h) }
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
