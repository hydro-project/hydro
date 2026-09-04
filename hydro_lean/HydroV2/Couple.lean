import HydroV2.Transfer
import HydroV2.SchedCausal

/-!
# HydroV2 · the coupling corner (`CoupleSem`) — derived decisions at an ambient horizon

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

namespace HydroV2

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

/-- Tick-singleton corner: coupled at every step `≤ T`. -/
structure CoTickSing (T : Nat) (n : Nat) (σ : Type) (b : SingBound σ) :
    Type where
  sr : Fin n → Nat → Trace σ
  rr : TickV n σ b
  wf : Prop
  cpl : wf → ∀ k, k ≤ T → ∀ i, sr i k <+: tickVals b rr i

/-- Tick-stream corner: coupled at every step `≤ T`. -/
structure CoTickStream (T : Nat) (n : Nat) (α : Type) [DecidableEq α]
    (ord : StrOrd) (ret : Retries) : Type where
  sr : Fin n → Nat → Trace (List α)
  rr : Fin n → Trace (PoolCarrier α ord ret)
  wf : Prop
  cpl : wf → ∀ k, k ≤ T → ∀ i, BatchTraceLe ord ret (sr i k) (rr i)

/-! ## Knot combinators (stream) -/

namespace CoStream

variable {T : Nat} {n : Nat} {α : Type} [DecidableEq α]
  {ord : StrOrd} {ret : Retries}

/-- Coupling at an earlier horizon, by machine-history restriction. -/
theorem cpl_le (C : CoStream T n α ord ret) (hwf : C.wf) {k : Nat}
    (hk : k ≤ T) (i : Fin n) :
    ListLe ord ret ((C.sr i).view k) (C.rr i) :=
  ListLe.of_prefix ((C.sr i).mono_le hk) (C.cpl hwf i)

/-- A shifted history's view is a prefix of the original's (delay only
loses recency). -/
theorem shift_view_prefix (x : StepHist α) :
    ∀ t, (x.shift).view t <+: x.view t
  | 0 => List.nil_prefix
  | t + 1 => by
    show x.view t <+: x.view (t + 1)
    exact x.mono t

/-- The knot-boundary shift: machine leg delayed one step, reader
kept. The coupling survives at the *same* ambient horizon because the
shifted view at `T` is an earlier view of the original. -/
def shiftC (C : CoStream T n α ord ret) : CoStream T n α ord ret where
  sr := fun i => (C.sr i).shift
  rr := C.rr
  wf := C.wf
  cpl := fun hwf i =>
    ListLe.of_prefix (shift_view_prefix (C.sr i) T) (C.cpl hwf i)

/-- The bottom corner (stage seed). -/
def botC : CoStream T n α ord ret where
  sr := fun _i => StepHist.bot
  rr := fun _i => PoolBot ord ret
  wf := True
  cpl := fun _ _i => ListLe.nil _ _ _

/-- Reader probe: both legs read `v`; never well-formed (the probes
only name the body's action on a leg). -/
def readEmbed (v : Fin n → PoolCarrier α ord ret) :
    CoStream T n α ord ret where
  sr := fun _i => StepHist.bot
  rr := v
  wf := False
  cpl := fun hwf => absurd hwf (by simp)

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

namespace CoTickSing

variable {T : Nat} {n : Nat} {σ : Type}

def shiftC (C : CoTickSing T n σ .unbounded) :
    CoTickSing T n σ .unbounded where
  sr := fun i u => match u with
    | 0 => ([] : Trace σ)
    | u + 1 => C.sr i u
  rr := C.rr
  wf := C.wf
  cpl := fun hwf k _hk i => by
    cases k with
    | zero => exact List.nil_prefix
    | succ u => exact C.cpl hwf u (by omega) i

def botC : CoTickSing T n σ .unbounded where
  sr := fun _i _t => ([] : Trace σ)
  rr := fun _i => ([] : Trace σ)
  wf := True
  cpl := fun _ _k _hk _i => List.nil_prefix

def readEmbed (v : TickV n σ .unbounded) :
    CoTickSing T n σ .unbounded where
  sr := fun _i _t => ([] : Trace σ)
  rr := v
  wf := False
  cpl := fun hwf => absurd hwf (by simp)

def schedEmbed (x : Fin n → Nat → Trace σ) :
    CoTickSing T n σ .unbounded where
  sr := x
  rr := fun _i => ([] : Trace σ)
  wf := False
  cpl := fun hwf => absurd hwf (by simp)

/-- External input (tick-singleton boundary). -/
def input (s : Fin n → Nat → Trace σ) (v : TickV n σ .unbounded)
    (hc : ∀ t i, s i t <+: v i) : CoTickSing T n σ .unbounded where
  sr := s
  rr := v
  wf := True
  cpl := fun _ k _hk i => hc k i

/-- Two-leg probe (tick). -/
def probe2 (x : Fin n → Nat → Trace σ) (v : TickV n σ .unbounded) :
    CoTickSing T n σ .unbounded where
  sr := x
  rr := v
  wf := False
  cpl := fun hwf => absurd hwf (by simp)

/-- Lower the couple horizon (tick couplings quantify all steps `≤ T`,
so restriction is inclusion). -/
def lower (C : CoTickSing T n σ .unbounded) {T' : Nat} (h : T' ≤ T) :
    CoTickSing T' n σ .unbounded where
  sr := C.sr
  rr := C.rr
  wf := C.wf
  cpl := fun hwf k hk i => C.cpl hwf k (hk.trans h) i

variable (body : CoTickSing T n σ .unbounded →
  CoTickSing T n σ .unbounded)

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

def vbody (v : TickV n σ .unbounded) : TickV n σ .unbounded :=
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

/-- Re-monotonization is a no-op on step-ascending families (local
copy of the `SquareHsat` fact, to keep `Couple` self-contained). -/
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
  (body : CoTickSing T n σ .unbounded → CoTickSing T n σ .unbounded)
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
      (v : TickV n σ .unbounded),
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
  TickSingleton ℓ σ b := CoTickSing Tc (mem ℓ) σ b
  TickStream ℓ α _ ord ret := CoTickStream Tc (mem ℓ) α ord ret
  TransportDec p c := Fin p → Fin c → Nat → Nat
  OrderSelDec _ _ := Unit
  SnapDec _ _ _ := Unit
  BatchDec _ _ := Unit
  OrdBatchDec _ := Unit
  BatchOrdSelDec n α := BatchOrderSelection n α
  SampleDec n := SampleTimes n
  TimerDec n := TimerVerdicts n
  PulseDec n := TimingPulses n
  EmitDec n β := Fin n → List (List β)
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
  broadcast dt s :=
    { sr := fun i j => (s.sr j).deliver (dt i j)
      rr := (Values L mem).broadcast () s.rr
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
        show _ <+: tickVals b ((Values L mem).snapshot (ℓ := ℓ)
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
  assume_ordering_batch bs sel :=
    { sr := bs.sr
      rr := (Values L mem).assume_ordering_batch bs.rr sel
      wf := False
      cpl := fun hwf => absurd hwf (by simp) }
  mapBatchWith bs t f :=
    { sr := fun i step => (Trace.zip (bs.sr i step) (t.sr i step)).map
        (fun bx => f i bx.1 bx.2)
      rr := (Values L mem).mapBatchWith bs.rr t.rr f
      wf := bs.wf ∧ t.wf
      cpl := fun hwf k hk i =>
        (zip_prefix (bs.cpl hwf.1 k hk i) (t.cpl hwf.2 k hk i)).map _ }
  mapBatch bs f :=
    { sr := fun i step => (bs.sr i step).map (f i)
      rr := (Values L mem).mapBatch bs.rr f
      wf := bs.wf
      cpl := fun hwf k hk i =>
        List.IsPrefix.map (f i) (bs.cpl hwf k hk i) }
  mapBatchesWith bs t f :=
    { sr := fun i step => (Trace.zip (bs.sr i step) (t.sr i step)).map
        (fun bx => bx.1.map (fun a => f i a bx.2))
      rr := (Values L mem).mapBatchesWith bs.rr t.rr f
      wf := bs.wf ∧ t.wf
      cpl := fun hwf k hk i => by
        show ((Trace.zip (bs.sr i k) (t.sr i k)).map
            (fun bx => bx.1.map (fun a => f i a bx.2))).map
            (fun b => Multiset.ofList b) <+: _
        refine List.IsPrefix.trans ?_
          ((zip_prefix (bs.cpl hwf.1 k hk i) (t.cpl hwf.2 k hk i)).map _)
        rw [map_ofList_batches
            (fun bx => bx.1.map (fun a => f i a bx.2))
            (fun bx => bx.1.map (fun a => f i a bx.2))
            (fun _ => rfl), ← zip_map_left'] }
  filterMapBatchesWith bs t f :=
    { sr := fun i step => (Trace.zip (bs.sr i step) (t.sr i step)).map
        (fun bx => bx.1.filterMap (fun a => f i a bx.2))
      rr := (Values L mem).filterMapBatchesWith bs.rr t.rr f
      wf := bs.wf ∧ t.wf
      cpl := fun hwf k hk i => by
        show ((Trace.zip (bs.sr i k) (t.sr i k)).map
            (fun bx => bx.1.filterMap (fun a => f i a bx.2))).map
            (fun b => Multiset.ofList b) <+: _
        refine List.IsPrefix.trans ?_
          ((zip_prefix (bs.cpl hwf.1 k hk i) (t.cpl hwf.2 k hk i)).map _)
        rw [map_ofList_batches
            (fun bx => bx.1.filterMap (fun a => f i a bx.2))
            (fun bx => bx.1.filterMap (fun a => f i a bx.2))
            (fun bx => (Multiset.filterMap_coe _ _).symm),
          ← zip_map_left'] }
  scan_batches_across_ticks bs t g init :=
    { sr := fun i step => scanAcrossTicksTrace
        (fun s bt => g i s bt.1 bt.2) init
        (Trace.zip (bs.sr i step) (t.sr i step))
      rr := (Values L mem).scan_batches_across_ticks bs.rr t.rr g init
      wf := bs.wf ∧ t.wf
      cpl := fun hwf k hk i => scanAcrossTicksTrace_prefix _ init
        (zip_prefix (bs.cpl hwf.1 k hk i) (t.cpl hwf.2 k hk i)) }
  fold_batches_across_ticks_monotone vo g init comm hinfl bs :=
    { sr := fun i step => foldAcrossTicksTrace
        (fun s b => b.foldl (g i) s) init (bs.sr i step)
      rr := (Values L mem).fold_batches_across_ticks_monotone
        vo g init comm hinfl bs.rr
      wf := bs.wf
      cpl := fun hwf k hk i => by
        refine List.IsPrefix.trans ?_
          (foldAcrossTicksTrace_prefix _ init (bs.cpl hwf k hk i))
        rw [foldAcrossTicksTrace_map]
        exact List.prefix_refl _ }
  scan_batches_unordered_across_ticks bs t g init :=
    { sr := fun i step => scanAcrossTicksTrace
        (fun s bt => g i s (Multiset.ofList bt.1) bt.2) init
        (Trace.zip (bs.sr i step) (t.sr i step))
      rr := (Values L mem).scan_batches_unordered_across_ticks
        bs.rr t.rr g init
      wf := bs.wf ∧ t.wf
      cpl := fun hwf k hk i => by
        refine List.IsPrefix.trans ?_
          (scanAcrossTicksTrace_prefix _ init
            (zip_prefix (bs.cpl hwf.1 k hk i) (t.cpl hwf.2 k hk i)))
        rw [zip_map_left', scanAcrossTicksTrace_map] }
  scan_batches_unordered bs g init :=
    { sr := fun i step => scanAcrossTicksTrace
        (fun s b => g i s (Multiset.ofList b)) init (bs.sr i step)
      rr := (Values L mem).scan_batches_unordered bs.rr g init
      wf := bs.wf
      cpl := fun hwf k hk i => by
        refine List.IsPrefix.trans ?_
          (scanAcrossTicksTrace_prefix _ init (bs.cpl hwf k hk i))
        rw [scanAcrossTicksTrace_map] }
  scan_batches_unordered₂ bs cs g init :=
    { sr := fun i step => scanAcrossTicksTrace
        (fun s bt => g i s (Multiset.ofList bt.1) (Multiset.ofList bt.2))
        init (Trace.zip (bs.sr i step) (cs.sr i step))
      rr := (Values L mem).scan_batches_unordered₂ bs.rr cs.rr g init
      wf := bs.wf ∧ cs.wf
      cpl := fun hwf k hk i => by
        refine List.IsPrefix.trans ?_
          (scanAcrossTicksTrace_prefix _ init
            (zip_prefix (bs.cpl hwf.1 k hk i) (cs.cpl hwf.2 k hk i)))
        rw [zip_map_right', zip_map_left', List.map_map,
          scanAcrossTicksTrace_map]
        exact List.prefix_refl _ }
  scan_across_ticks t g init :=
    { sr := fun i step => scanAcrossTicksTrace (g i) init (t.sr i step)
      rr := (Values L mem).scan_across_ticks t.rr g init
      wf := t.wf
      cpl := fun hwf k hk i =>
        scanAcrossTicksTrace_prefix _ init (t.cpl hwf k hk i) }
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
  fold_across_ticks_monotone vo g init hinfl s :=
    { sr := fun i step => foldAcrossTicksTrace (g i) init (s.sr i step)
      rr := (Values L mem).fold_across_ticks_monotone vo g init hinfl
        s.rr
      wf := s.wf
      cpl := fun hwf k hk i =>
        foldAcrossTicksTrace_prefix _ init (s.cpl hwf k hk i) }
  mapMonotone vo' m h hpres :=
    { sr := fun i step => (m.sr i step).map (h i)
      rr := (Values L mem).mapMonotone vo' m.rr h hpres
      wf := m.wf
      cpl := fun hwf k hk i =>
        List.IsPrefix.map (h i) (m.cpl hwf k hk i) }
  forgetBound m :=
    { sr := m.sr
      rr := (Values L mem).forgetBound m.rr
      wf := m.wf
      cpl := fun hwf k hk i => m.cpl hwf k hk i }
  defer init t :=
    { sr := fun i step => init :: t.sr i step
      rr := (Values L mem).defer init t.rr
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
  mapBatchesUnordered bs t f :=
    { sr := fun i step => (Trace.zip (bs.sr i step) (t.sr i step)).map
        (fun bx => f i (Multiset.ofList bx.1) bx.2)
      rr := (Values L mem).mapBatchesUnordered bs.rr t.rr f
      wf := bs.wf ∧ t.wf
      cpl := fun hwf k hk i => by
        refine List.IsPrefix.trans ?_
          ((zip_prefix (bs.cpl hwf.1 k hk i) (t.cpl hwf.2 k hk i)).map _)
        rw [zip_map_left', List.map_map]
        exact List.prefix_refl _ }
  emitBatches t :=
    { sr := t.sr
      rr := (Values L mem).emitBatches t.rr
      wf := t.wf
      cpl := fun hwf k hk i => t.cpl hwf k hk i }
  emitMultisetBatches t e :=
    { sr := fun i step => emitLin (t.sr i step) (e i)
      rr := (Values L mem).emitMultisetBatches t.rr ()
      wf := t.wf
      cpl := fun hwf k hk i =>
        (emitLin_map_ofList_prefix _ _).trans (t.cpl hwf k hk i) }
  emitBatchesUnordered t :=
    { sr := t.sr
      rr := (Values L mem).emitBatchesUnordered t.rr
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
              (v : TickV (mem ℓ) σ .unbounded),
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

end HydroV2
