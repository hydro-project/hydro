import Hydro.Couple


/-!
# Hydro · coupling-corner projections (`CoupleProj`)

Per-op `rfl` projection lemmas for `CoupleSem` (the D39 corner): for
every op, its output's `rr` is the `Values` op at the self-derived
decision, its `sr` is the `SchedSem` op at the machine decision, and
its `wf` is the conjunction of its inputs' — proved once, at variable
arguments (never per program). `co_transfer [defs]` closes
`rr`/`sr`-naming identities by `simp only` + reducible `rfl`;
`co_wf [defs]` normalizes a program's residual `wf` to its knots'
components.
-/

namespace Hydro

variable {L : Type} {mem : L → Nat}
  {pacing : (ℓ : L) → Fin (mem ℓ) → Nat → Bool} {Tc Td : Nat}
  {hjT : Tc ≤ Td} {ℓ : L}

/-! ## Boundary constructors (instance-projected types)

Program-facing spellings: the raw-record constructors of `Couple.lean`
restated at the instance-projected carrier types, so the projection
lemmas' keyed matching sees one spelling (FINDINGS D33/D42). -/

section Boundary

/-- A machine input wire coupled below a denotational pool. -/
def CoStream.inputC {α : Type} [DecidableEq α] {ord : StrOrd}
    {ret : Retries} (h : (SchedSem L mem pacing).Stream ℓ α ord ret)
    (v : (Values L mem).Stream ℓ α ord ret)
    (hc : ∀ t i, ListLe ord ret ((h i).view t) (v i)) :
    (CoupleSem L mem pacing Tc Td hjT).Stream ℓ α ord ret :=
  CoStream.input h v hc

/-- A tick input wire coupled below a denotational trace. -/
def CoTickSing.inputC {σ : Type}
    (s : (SchedSem L mem pacing).Ticked ℓ σ)
    (v : (Values L mem).Ticked ℓ σ)
    (hc : ∀ t i, s i t <+: v i) :
    (CoupleSem L mem pacing Tc Td hjT).Ticked ℓ σ :=
  CoTickSing.input s v hc

/-- An async-singleton input wire: both legs quote a common fold over
a source dominated by a common pool (the boundary form of the
fold-provenance `CoSing.cpl` carries), at every horizon. -/
def CoSing.inputC {α σ : Type} [DecidableEq α] {ord : StrOrd}
    {ret : Retries} {b : SingBound σ}
    (h : (SchedSem L mem pacing).Singleton ℓ α σ ord ret b)
    (v : (Values L mem).Singleton ℓ α σ ord ret b)
    (hc : ∃ (g : σ → α → σ) (init : σ) (ok : FoldOkP ord ret g)
      (pool : Fin (mem ℓ) → PoolCarrier α ord ret),
      (∀ i, singReads b v i = snapTrace ord ret g init ok (pool i)) ∧
      (∀ i, (h i).read = fun l => l.foldl g init) ∧
      (∀ t i, ListLe ord ret ((h i).src.view t) (pool i))) :
    (CoupleSem L mem pacing Tc Td hjT).Singleton ℓ α σ ord ret b :=
  { sr := h, rr := v, wf := True,
    cpl := fun _ => by
      obtain ⟨g, init, ok, pool, hr, hf, hsrc⟩ := hc
      exact ⟨g, init, ok, pool, hr, hf, fun i => hsrc Tc i⟩ }

/-- A coupled carrier from a single-horizon coupling (the graded
knot obligations construct these at lowered instances). -/
def CoStream.mkC {α : Type} [DecidableEq α] {ord : StrOrd}
    {ret : Retries} (x : (SchedSem L mem pacing).Stream ℓ α ord ret)
    (v : (Values L mem).Stream ℓ α ord ret)
    (hc : ∀ i, ListLe ord ret ((x i).view Tc) (v i)) :
    (CoupleSem L mem pacing Tc Td hjT).Stream ℓ α ord ret :=
  { sr := x, rr := v, wf := True, cpl := fun _ i => hc i }

def CoTickSing.mkC {σ : Type}
    (x : (SchedSem L mem pacing).Ticked ℓ σ)
    (v : (Values L mem).Ticked ℓ σ)
    (hc : ∀ k, k ≤ Tc → ∀ i, x i k <+: v i) :
    (CoupleSem L mem pacing Tc Td hjT).Ticked ℓ σ :=
  { sr := x, rr := v, wf := True, cpl := fun _ k hk i => hc k hk i }

theorem co_mkC_rr {α : Type} [DecidableEq α] {ord : StrOrd}
    {ret : Retries} (x : (SchedSem L mem pacing).Stream ℓ α ord ret)
    (v : (Values L mem).Stream ℓ α ord ret)
    (hc : ∀ i, ListLe ord ret ((x i).view Tc) (v i)) :
    (CoStream.mkC (Td := Td) (hjT := hjT) x v hc).rr = v := rfl

theorem co_mkC_sr {α : Type} [DecidableEq α] {ord : StrOrd}
    {ret : Retries} (x : (SchedSem L mem pacing).Stream ℓ α ord ret)
    (v : (Values L mem).Stream ℓ α ord ret)
    (hc : ∀ i, ListLe ord ret ((x i).view Tc) (v i)) :
    (CoStream.mkC (Td := Td) (hjT := hjT) x v hc).sr = x := rfl

theorem co_mkC_wf {α : Type} [DecidableEq α] {ord : StrOrd}
    {ret : Retries} (x : (SchedSem L mem pacing).Stream ℓ α ord ret)
    (v : (Values L mem).Stream ℓ α ord ret)
    (hc : ∀ i, ListLe ord ret ((x i).view Tc) (v i)) :
    (CoStream.mkC (Td := Td) (hjT := hjT) x v hc).wf = True := rfl

theorem co_tick_mkC_rr {σ : Type}
    (x : (SchedSem L mem pacing).Ticked ℓ σ)
    (v : (Values L mem).Ticked ℓ σ)
    (hc : ∀ k, k ≤ Tc → ∀ i, x i k <+: v i) :
    (CoTickSing.mkC (Td := Td) (hjT := hjT) x v hc).rr = v := rfl

theorem co_tick_mkC_sr {σ : Type}
    (x : (SchedSem L mem pacing).Ticked ℓ σ)
    (v : (Values L mem).Ticked ℓ σ)
    (hc : ∀ k, k ≤ Tc → ∀ i, x i k <+: v i) :
    (CoTickSing.mkC (Td := Td) (hjT := hjT) x v hc).sr = x := rfl

theorem co_tick_mkC_wf {σ : Type}
    (x : (SchedSem L mem pacing).Ticked ℓ σ)
    (v : (Values L mem).Ticked ℓ σ)
    (hc : ∀ k, k ≤ Tc → ∀ i, x i k <+: v i) :
    (CoTickSing.mkC (Td := Td) (hjT := hjT) x v hc).wf = True := rfl

/-! Raw-record twins (the knot combinators `sbody`/`vbody`/`fixSr`
are defined over the raw probes; their unfoldings surface these
spellings). -/

theorem co_embedS_sr_raw {n : Nat} {α : Type} [DecidableEq α]
    {ord : StrOrd} {ret : Retries} {T : Nat} (x : Fin n → StepHist α) :
    (CoStream.schedEmbed (T := T) (ord := ord) (ret := ret) x).sr = x
    := rfl

theorem co_tick_embedS_sr_raw {n : Nat} {σ : Type} {T : Nat}
    (x : Fin n → Nat → Trace σ) :
    (CoTickSing.schedEmbed (T := T) x).sr = x := rfl

theorem co_probe2_rr_raw {n : Nat} {α : Type} [DecidableEq α]
    {ord : StrOrd} {ret : Retries} {T : Nat} (x : Fin n → StepHist α)
    (v : Fin n → PoolCarrier α ord ret) :
    (CoStream.probe2 (T := T) x v).rr = v := rfl

theorem co_probe2_sr_raw {n : Nat} {α : Type} [DecidableEq α]
    {ord : StrOrd} {ret : Retries} {T : Nat} (x : Fin n → StepHist α)
    (v : Fin n → PoolCarrier α ord ret) :
    (CoStream.probe2 (T := T) x v).sr = x := rfl

theorem co_tick_probe2_rr_raw {n : Nat} {σ : Type} {T : Nat}
    (x : Fin n → Nat → Trace σ) (v : TickV n σ) :
    (CoTickSing.probe2 (T := T) x v).rr = v := rfl

theorem co_tick_probe2_sr_raw {n : Nat} {σ : Type} {T : Nat}
    (x : Fin n → Nat → Trace σ) (v : TickV n σ) :
    (CoTickSing.probe2 (T := T) x v).sr = x := rfl

/-! Raw-binder twins of the `mkC` projections (the knot `wf`'s graded
obligation binds its legs at the raw carrier types, and `simp`'s
metavariable assignments type-check at reducible transparency). -/

theorem co_mkC_rr_raw {α : Type} [DecidableEq α] {ord : StrOrd}
    {ret : Retries} (x : Fin (mem ℓ) → StepHist α)
    (v : Fin (mem ℓ) → PoolCarrier α ord ret)
    (hc : ∀ i, ListLe ord ret ((x i).view Tc) (v i)) :
    (CoStream.mkC (Td := Td) (hjT := hjT) (pacing := pacing)
      x v hc).rr = v := rfl

theorem co_mkC_sr_raw {α : Type} [DecidableEq α] {ord : StrOrd}
    {ret : Retries} (x : Fin (mem ℓ) → StepHist α)
    (v : Fin (mem ℓ) → PoolCarrier α ord ret)
    (hc : ∀ i, ListLe ord ret ((x i).view Tc) (v i)) :
    (CoStream.mkC (Td := Td) (hjT := hjT) (pacing := pacing)
      x v hc).sr = x := rfl

theorem co_mkC_wf_raw {α : Type} [DecidableEq α] {ord : StrOrd}
    {ret : Retries} (x : Fin (mem ℓ) → StepHist α)
    (v : Fin (mem ℓ) → PoolCarrier α ord ret)
    (hc : ∀ i, ListLe ord ret ((x i).view Tc) (v i)) :
    (CoStream.mkC (Td := Td) (hjT := hjT) (pacing := pacing)
      x v hc).wf = True := rfl

theorem co_tick_mkC_rr_raw {σ : Type}
    (x : Fin (mem ℓ) → Nat → Trace σ) (v : TickV (mem ℓ) σ)
    (hc : ∀ k, k ≤ Tc → ∀ i, x i k <+: v i) :
    (CoTickSing.mkC (Td := Td) (hjT := hjT) (pacing := pacing)
      x v hc).rr = v := rfl

theorem co_tick_mkC_sr_raw {σ : Type}
    (x : Fin (mem ℓ) → Nat → Trace σ) (v : TickV (mem ℓ) σ)
    (hc : ∀ k, k ≤ Tc → ∀ i, x i k <+: v i) :
    (CoTickSing.mkC (Td := Td) (hjT := hjT) (pacing := pacing)
      x v hc).sr = x := rfl

theorem co_tick_mkC_wf_raw {σ : Type}
    (x : Fin (mem ℓ) → Nat → Trace σ) (v : TickV (mem ℓ) σ)
    (hc : ∀ k, k ≤ Tc → ∀ i, x i k <+: v i) :
    (CoTickSing.mkC (Td := Td) (hjT := hjT) (pacing := pacing)
      x v hc).wf = True := rfl

/-- The two-leg probe at the instance type. -/
def CoStream.probeC {α : Type} [DecidableEq α] {ord : StrOrd}
    {ret : Retries} (x : (SchedSem L mem pacing).Stream ℓ α ord ret)
    (v : (Values L mem).Stream ℓ α ord ret) :
    (CoupleSem L mem pacing Tc Td hjT).Stream ℓ α ord ret :=
  CoStream.probe2 x v

def CoTickSing.probeC {σ : Type}
    (x : (SchedSem L mem pacing).Ticked ℓ σ)
    (v : (Values L mem).Ticked ℓ σ) :
    (CoupleSem L mem pacing Tc Td hjT).Ticked ℓ σ :=
  CoTickSing.probe2 x v

/-- Machine/sched probe at the instance type. -/
def CoStream.schedC {α : Type} [DecidableEq α] {ord : StrOrd}
    {ret : Retries} (x : (SchedSem L mem pacing).Stream ℓ α ord ret) :
    (CoupleSem L mem pacing Tc Td hjT).Stream ℓ α ord ret :=
  CoStream.schedEmbed x

def CoTickSing.schedC {σ : Type}
    (x : (SchedSem L mem pacing).Ticked ℓ σ) :
    (CoupleSem L mem pacing Tc Td hjT).Ticked ℓ σ :=
  CoTickSing.schedEmbed x

/-- Horizon lowering at the instance types. -/
def CoStream.lowerC {α : Type} [DecidableEq α] {ord : StrOrd}
    {ret : Retries} {Tc' : Nat} (h' : Tc' ≤ Tc) (h'' : Tc' ≤ Td)
    (C : (CoupleSem L mem pacing Tc Td hjT).Stream ℓ α ord ret) :
    (CoupleSem L mem pacing Tc' Td h'').Stream ℓ α ord ret :=
  CoStream.lower C h'

def CoTickSing.lowerC {σ : Type} {Tc' : Nat} (h' : Tc' ≤ Tc)
    (h'' : Tc' ≤ Td)
    (C : (CoupleSem L mem pacing Tc Td hjT).Ticked ℓ σ) :
    (CoupleSem L mem pacing Tc' Td h'').Ticked ℓ σ :=
  CoTickSing.lower C h'

def CoSing.lowerC {α σ : Type} [DecidableEq α] {ord : StrOrd}
    {ret : Retries} {b : SingBound σ} {Tc' : Nat} (h' : Tc' ≤ Tc)
    (h'' : Tc' ≤ Td)
    (C : (CoupleSem L mem pacing Tc Td hjT).Singleton ℓ α σ ord ret b) :
    (CoupleSem L mem pacing Tc' Td h'').Singleton ℓ α σ ord ret b :=
  CoSing.lower C h'

/-- Decision constructors at the instance types (`Unit` content sites;
machine data at cursor/timing/emission sites). -/
@[reducible] def CoDec.cursors {p c : Nat}
    (dt : (SchedSem L mem pacing).TransportDec p c) :
    (CoupleSem L mem pacing Tc Td hjT).TransportDec p c := dt

@[reducible] def CoDec.orderSel {n : Nat} (α : Type) :
    (CoupleSem L mem pacing Tc Td hjT).OrderSelDec n α := ()

@[reducible] def CoDec.snap {n : Nat} (α : Type) (ord : StrOrd) :
    (CoupleSem L mem pacing Tc Td hjT).SnapDec n α ord := ()

@[reducible] def CoDec.batch {n : Nat} (α : Type) :
    (CoupleSem L mem pacing Tc Td hjT).BatchDec n α := ()

@[reducible] def CoDec.ordBatch {n : Nat} :
    (CoupleSem L mem pacing Tc Td hjT).OrdBatchDec n := ()

@[reducible] def CoDec.sample {n : Nat} (times : SampleTimes n) :
    (CoupleSem L mem pacing Tc Td hjT).SampleDec n := times

@[reducible] def CoDec.timer {n : Nat} (verd : TimerVerdicts n) :
    (CoupleSem L mem pacing Tc Td hjT).TimerDec n := verd

@[reducible] def CoDec.pulse {n : Nat} (pulses : TimingPulses n) :
    (CoupleSem L mem pacing Tc Td hjT).PulseDec n := pulses

@[reducible] def CoDec.fix :
    (CoupleSem L mem pacing Tc Td hjT).FixDec := ()

theorem co_input_rr {α : Type} [DecidableEq α]
    {ord : StrOrd} {ret : Retries}
    (h : (SchedSem L mem pacing).Stream ℓ α ord ret)
    (v : (Values L mem).Stream ℓ α ord ret)
    (hc : ∀ t i, ListLe ord ret ((h i).view t) (v i)) :
    (CoStream.inputC (Tc := Tc) (Td := Td) (hjT := hjT)
      (pacing := pacing) h v hc).rr = v := rfl

theorem co_input_sr {α : Type} [DecidableEq α]
    {ord : StrOrd} {ret : Retries}
    (h : (SchedSem L mem pacing).Stream ℓ α ord ret)
    (v : (Values L mem).Stream ℓ α ord ret)
    (hc : ∀ t i, ListLe ord ret ((h i).view t) (v i)) :
    (CoStream.inputC (Tc := Tc) (Td := Td) (hjT := hjT)
      (pacing := pacing) h v hc).sr = h := rfl

theorem co_input_wf {α : Type} [DecidableEq α]
    {ord : StrOrd} {ret : Retries}
    (h : (SchedSem L mem pacing).Stream ℓ α ord ret)
    (v : (Values L mem).Stream ℓ α ord ret)
    (hc : ∀ t i, ListLe ord ret ((h i).view t) (v i)) :
    (CoStream.inputC (Tc := Tc) (Td := Td) (hjT := hjT)
      (pacing := pacing) h v hc).wf = True := rfl

theorem co_tick_input_rr {σ : Type}
    (s : (SchedSem L mem pacing).Ticked ℓ σ)
    (v : (Values L mem).Ticked ℓ σ)
    (hc : ∀ t i, s i t <+: v i) :
    (CoTickSing.inputC (Tc := Tc) (Td := Td) (hjT := hjT)
      (pacing := pacing) s v hc).rr = v := rfl

theorem co_tick_input_sr {σ : Type}
    (s : (SchedSem L mem pacing).Ticked ℓ σ)
    (v : (Values L mem).Ticked ℓ σ)
    (hc : ∀ t i, s i t <+: v i) :
    (CoTickSing.inputC (Tc := Tc) (Td := Td) (hjT := hjT)
      (pacing := pacing) s v hc).sr = s := rfl

theorem co_tick_input_wf {σ : Type}
    (s : (SchedSem L mem pacing).Ticked ℓ σ)
    (v : (Values L mem).Ticked ℓ σ)
    (hc : ∀ t i, s i t <+: v i) :
    (CoTickSing.inputC (Tc := Tc) (Td := Td) (hjT := hjT)
      (pacing := pacing) s v hc).wf = True := rfl

theorem co_probeC_rr {α : Type} [DecidableEq α]
    {ord : StrOrd} {ret : Retries}
    (x : (SchedSem L mem pacing).Stream ℓ α ord ret)
    (v : (Values L mem).Stream ℓ α ord ret) :
    (CoStream.probeC (Tc := Tc) (Td := Td) (hjT := hjT) x v).rr = v
    := rfl

theorem co_probeC_sr {α : Type} [DecidableEq α]
    {ord : StrOrd} {ret : Retries}
    (x : (SchedSem L mem pacing).Stream ℓ α ord ret)
    (v : (Values L mem).Stream ℓ α ord ret) :
    (CoStream.probeC (Tc := Tc) (Td := Td) (hjT := hjT) x v).sr = x
    := rfl

theorem co_tick_probeC_rr {σ : Type}
    (x : (SchedSem L mem pacing).Ticked ℓ σ)
    (v : (Values L mem).Ticked ℓ σ) :
    (CoTickSing.probeC (Tc := Tc) (Td := Td) (hjT := hjT) x v).rr = v
    := rfl

theorem co_tick_probeC_sr {σ : Type}
    (x : (SchedSem L mem pacing).Ticked ℓ σ)
    (v : (Values L mem).Ticked ℓ σ) :
    (CoTickSing.probeC (Tc := Tc) (Td := Td) (hjT := hjT) x v).sr = x
    := rfl

theorem co_schedC_sr {α : Type} [DecidableEq α]
    {ord : StrOrd} {ret : Retries}
    (x : (SchedSem L mem pacing).Stream ℓ α ord ret) :
    (CoStream.schedC (Tc := Tc) (Td := Td) (hjT := hjT) x).sr = x
    := rfl

theorem co_tick_schedC_sr {σ : Type}
    (x : (SchedSem L mem pacing).Ticked ℓ σ) :
    (CoTickSing.schedC (Tc := Tc) (Td := Td) (hjT := hjT) x).sr = x
    := rfl

theorem co_lowerC_rr {α : Type} [DecidableEq α] {ord : StrOrd}
    {ret : Retries} {Tc' : Nat} (h' : Tc' ≤ Tc) (h'' : Tc' ≤ Td)
    (C : (CoupleSem L mem pacing Tc Td hjT).Stream ℓ α ord ret) :
    (CoStream.lowerC h' h'' C).rr = C.rr := rfl

theorem co_lowerC_sr {α : Type} [DecidableEq α] {ord : StrOrd}
    {ret : Retries} {Tc' : Nat} (h' : Tc' ≤ Tc) (h'' : Tc' ≤ Td)
    (C : (CoupleSem L mem pacing Tc Td hjT).Stream ℓ α ord ret) :
    (CoStream.lowerC h' h'' C).sr = C.sr := rfl

theorem co_sing_lowerC_rr {α σ : Type} [DecidableEq α]
    {ord : StrOrd} {ret : Retries} {b : SingBound σ} {Tc' : Nat}
    (h' : Tc' ≤ Tc) (h'' : Tc' ≤ Td)
    (C : (CoupleSem L mem pacing Tc Td hjT).Singleton ℓ α σ ord ret
      b) :
    (CoSing.lowerC h' h'' C).rr = C.rr := rfl

theorem co_sing_lowerC_sr {α σ : Type} [DecidableEq α]
    {ord : StrOrd} {ret : Retries} {b : SingBound σ} {Tc' : Nat}
    (h' : Tc' ≤ Tc) (h'' : Tc' ≤ Td)
    (C : (CoupleSem L mem pacing Tc Td hjT).Singleton ℓ α σ ord ret
      b) :
    (CoSing.lowerC h' h'' C).sr = C.sr := rfl

theorem co_tick_lowerC_rr {σ : Type} {Tc' : Nat} (h' : Tc' ≤ Tc)
    (h'' : Tc' ≤ Td)
    (C : (CoupleSem L mem pacing Tc Td hjT).Ticked ℓ σ) :
    (CoTickSing.lowerC h' h'' C).rr = C.rr := rfl

theorem co_tick_lowerC_sr {σ : Type} {Tc' : Nat} (h' : Tc' ≤ Tc)
    (h'' : Tc' ≤ Td)
    (C : (CoupleSem L mem pacing Tc Td hjT).Ticked ℓ σ) :
    (CoTickSing.lowerC h' h'' C).sr = C.sr := rfl

end Boundary

/-! ## Per-op projections -/

section Ops

theorem co_map_rr {α β : Type} [DecidableEq α] [DecidableEq β]
    {ord : StrOrd}
    (s : (CoupleSem L mem pacing Tc Td hjT).Stream ℓ α ord .exactlyOnce)
    (f : Fin (mem ℓ) → α → β) :
    ((CoupleSem L mem pacing Tc Td hjT).map s f).rr
      = (Values L mem).map s.rr f := rfl

theorem co_map_sr {α β : Type} [DecidableEq α] [DecidableEq β]
    {ord : StrOrd}
    (s : (CoupleSem L mem pacing Tc Td hjT).Stream ℓ α ord .exactlyOnce)
    (f : Fin (mem ℓ) → α → β) :
    ((CoupleSem L mem pacing Tc Td hjT).map s f).sr
      = (SchedSem L mem pacing).map (ℓ := ℓ) (ord := ord) s.sr f := rfl

theorem co_map_wf {α β : Type} [DecidableEq α] [DecidableEq β]
    {ord : StrOrd}
    (s : (CoupleSem L mem pacing Tc Td hjT).Stream ℓ α ord .exactlyOnce)
    (f : Fin (mem ℓ) → α → β) :
    ((CoupleSem L mem pacing Tc Td hjT).map s f).wf = s.wf := rfl

theorem co_filterMap_rr {α β : Type} [DecidableEq α] [DecidableEq β]
    {ord : StrOrd}
    (s : (CoupleSem L mem pacing Tc Td hjT).Stream ℓ α ord .exactlyOnce)
    (f : Fin (mem ℓ) → α → Option β) :
    ((CoupleSem L mem pacing Tc Td hjT).filterMap s f).rr
      = (Values L mem).filterMap s.rr f := rfl

theorem co_filterMap_sr {α β : Type} [DecidableEq α] [DecidableEq β]
    {ord : StrOrd}
    (s : (CoupleSem L mem pacing Tc Td hjT).Stream ℓ α ord .exactlyOnce)
    (f : Fin (mem ℓ) → α → Option β) :
    ((CoupleSem L mem pacing Tc Td hjT).filterMap s f).sr
      = (SchedSem L mem pacing).filterMap (ℓ := ℓ) (ord := ord) s.sr f
      := rfl

theorem co_filterMap_wf {α β : Type} [DecidableEq α] [DecidableEq β]
    {ord : StrOrd}
    (s : (CoupleSem L mem pacing Tc Td hjT).Stream ℓ α ord .exactlyOnce)
    (f : Fin (mem ℓ) → α → Option β) :
    ((CoupleSem L mem pacing Tc Td hjT).filterMap s f).wf = s.wf := rfl

theorem co_broadcast_closed_rr {c p : L} {α : Type} [DecidableEq α]
    {ord : StrOrd} {ret : Retries}
    (dt : (CoupleSem L mem pacing Tc Td hjT).TransportDec (mem p) (mem c))
    (s : (CoupleSem L mem pacing Tc Td hjT).Stream c α ord ret) :
    ((CoupleSem L mem pacing Tc Td hjT).broadcast_closed (p := p) dt s).rr
      = (Values L mem).broadcast_closed (p := p) () s.rr := rfl

theorem co_broadcast_closed_sr {c p : L} {α : Type} [DecidableEq α]
    {ord : StrOrd} {ret : Retries}
    (dt : (CoupleSem L mem pacing Tc Td hjT).TransportDec (mem p) (mem c))
    (s : (CoupleSem L mem pacing Tc Td hjT).Stream c α ord ret) :
    ((CoupleSem L mem pacing Tc Td hjT).broadcast_closed (p := p) dt s).sr
      = (SchedSem L mem pacing).broadcast_closed (c := c) (p := p) (ord := ord)
          (ret := ret) dt s.sr := rfl

theorem co_broadcast_closed_wf {c p : L} {α : Type} [DecidableEq α]
    {ord : StrOrd} {ret : Retries}
    (dt : (CoupleSem L mem pacing Tc Td hjT).TransportDec (mem p) (mem c))
    (s : (CoupleSem L mem pacing Tc Td hjT).Stream c α ord ret) :
    ((CoupleSem L mem pacing Tc Td hjT).broadcast_closed (p := p) dt s).wf = s.wf
    := rfl

theorem co_demux_rr {c p : L} {α : Type} [DecidableEq α]
    {ord : StrOrd}
    (dt : (CoupleSem L mem pacing Tc Td hjT).TransportDec (mem p) (mem c))
    (s : (CoupleSem L mem pacing Tc Td hjT).Stream c (Nat × α) ord
      .exactlyOnce) (addr : Fin (mem p) → Nat) :
    ((CoupleSem L mem pacing Tc Td hjT).demux dt s addr).rr
      = (Values L mem).demux () s.rr addr := rfl

theorem co_demux_sr {c p : L} {α : Type} [DecidableEq α]
    {ord : StrOrd}
    (dt : (CoupleSem L mem pacing Tc Td hjT).TransportDec (mem p) (mem c))
    (s : (CoupleSem L mem pacing Tc Td hjT).Stream c (Nat × α) ord
      .exactlyOnce) (addr : Fin (mem p) → Nat) :
    ((CoupleSem L mem pacing Tc Td hjT).demux dt s addr).sr
      = (SchedSem L mem pacing).demux (c := c) (p := p) (ord := ord)
          dt s.sr addr := rfl

theorem co_demux_wf {c p : L} {α : Type} [DecidableEq α]
    {ord : StrOrd}
    (dt : (CoupleSem L mem pacing Tc Td hjT).TransportDec (mem p) (mem c))
    (s : (CoupleSem L mem pacing Tc Td hjT).Stream c (Nat × α) ord
      .exactlyOnce) (addr : Fin (mem p) → Nat) :
    ((CoupleSem L mem pacing Tc Td hjT).demux dt s addr).wf = s.wf := rfl

theorem co_values_rr {p c : L} {α : Type} [DecidableEq α]
    {ord : StrOrd} {ret : Retries}
    (k : (CoupleSem L mem pacing Tc Td hjT).KeyedStream p c α ord ret) :
    ((CoupleSem L mem pacing Tc Td hjT).values k).rr
      = (Values L mem).values k.rr := rfl

theorem co_values_sr {p c : L} {α : Type} [DecidableEq α]
    {ord : StrOrd} {ret : Retries}
    (k : (CoupleSem L mem pacing Tc Td hjT).KeyedStream p c α ord ret) :
    ((CoupleSem L mem pacing Tc Td hjT).values k).sr
      = (SchedSem L mem pacing).values (p := p) (c := c) (ord := ord)
          (ret := ret) k.sr := rfl

theorem co_values_wf {p c : L} {α : Type} [DecidableEq α]
    {ord : StrOrd} {ret : Retries}
    (k : (CoupleSem L mem pacing Tc Td hjT).KeyedStream p c α ord ret) :
    ((CoupleSem L mem pacing Tc Td hjT).values k).wf = k.wf := rfl

theorem co_weaken_retries_rr {α : Type} [DecidableEq α]
    {ord : StrOrd}
    (s : (CoupleSem L mem pacing Tc Td hjT).Stream ℓ α ord .exactlyOnce) :
    ((CoupleSem L mem pacing Tc Td hjT).weaken_retries s).rr
      = (Values L mem).weaken_retries s.rr := rfl

theorem co_weaken_retries_sr {α : Type} [DecidableEq α]
    {ord : StrOrd}
    (s : (CoupleSem L mem pacing Tc Td hjT).Stream ℓ α ord .exactlyOnce) :
    ((CoupleSem L mem pacing Tc Td hjT).weaken_retries s).sr
      = (SchedSem L mem pacing).weaken_retries (ℓ := ℓ) (ord := ord)
          s.sr := rfl

theorem co_weaken_retries_wf {α : Type} [DecidableEq α]
    {ord : StrOrd}
    (s : (CoupleSem L mem pacing Tc Td hjT).Stream ℓ α ord .exactlyOnce) :
    ((CoupleSem L mem pacing Tc Td hjT).weaken_retries s).wf = s.wf := rfl

theorem co_union_rr {α : Type} [DecidableEq α] {ret : Retries}
    (a b : (CoupleSem L mem pacing Tc Td hjT).Stream ℓ α .noOrder ret) :
    ((CoupleSem L mem pacing Tc Td hjT).union a b).rr
      = (Values L mem).union a.rr b.rr := rfl

theorem co_union_sr {α : Type} [DecidableEq α] {ret : Retries}
    (a b : (CoupleSem L mem pacing Tc Td hjT).Stream ℓ α .noOrder ret) :
    ((CoupleSem L mem pacing Tc Td hjT).union a b).sr
      = (SchedSem L mem pacing).union (ℓ := ℓ) (ret := ret) a.sr b.sr
      := rfl

theorem co_union_wf {α : Type} [DecidableEq α] {ret : Retries}
    (a b : (CoupleSem L mem pacing Tc Td hjT).Stream ℓ α .noOrder ret) :
    ((CoupleSem L mem pacing Tc Td hjT).union a b).wf = (a.wf ∧ b.wf) := rfl

theorem co_assume_ordering_rr {α : Type} [DecidableEq α]
    (u : (CoupleSem L mem pacing Tc Td hjT).Stream ℓ α .noOrder .exactlyOnce)
    (sel : (CoupleSem L mem pacing Tc Td hjT).OrderSelDec (mem ℓ) α) :
    ((CoupleSem L mem pacing Tc Td hjT).assume_ordering u sel).rr
      = (Values L mem).assume_ordering u.rr (ordSelDerive Td u.sr)
    := rfl

theorem co_assume_ordering_sr {α : Type} [DecidableEq α]
    (u : (CoupleSem L mem pacing Tc Td hjT).Stream ℓ α .noOrder .exactlyOnce)
    (sel : (CoupleSem L mem pacing Tc Td hjT).OrderSelDec (mem ℓ) α) :
    ((CoupleSem L mem pacing Tc Td hjT).assume_ordering u sel).sr
      = (SchedSem L mem pacing).assume_ordering (ℓ := ℓ) u.sr () := rfl

theorem co_assume_ordering_wf {α : Type} [DecidableEq α]
    (u : (CoupleSem L mem pacing Tc Td hjT).Stream ℓ α .noOrder .exactlyOnce)
    (sel : (CoupleSem L mem pacing Tc Td hjT).OrderSelDec (mem ℓ) α) :
    ((CoupleSem L mem pacing Tc Td hjT).assume_ordering u sel).wf = u.wf := rfl

theorem co_fold_rr {α σ : Type} [DecidableEq α] {ord : StrOrd}
    {ret : Retries} (g : σ → α → σ) (init : σ) (ok : FoldOk ord ret g)
    (s : (CoupleSem L mem pacing Tc Td hjT).Stream ℓ α ord ret) :
    ((CoupleSem L mem pacing Tc Td hjT).fold g init ok s).rr
      = (Values L mem).fold g init ok s.rr := rfl

theorem co_fold_sr {α σ : Type} [DecidableEq α] {ord : StrOrd}
    {ret : Retries} (g : σ → α → σ) (init : σ) (ok : FoldOk ord ret g)
    (s : (CoupleSem L mem pacing Tc Td hjT).Stream ℓ α ord ret) :
    ((CoupleSem L mem pacing Tc Td hjT).fold g init ok s).sr
      = (SchedSem L mem pacing).fold (ℓ := ℓ) (ord := ord) (ret := ret)
          g init ok s.sr := rfl

theorem co_fold_wf {α σ : Type} [DecidableEq α] {ord : StrOrd}
    {ret : Retries} (g : σ → α → σ) (init : σ) (ok : FoldOk ord ret g)
    (s : (CoupleSem L mem pacing Tc Td hjT).Stream ℓ α ord ret) :
    ((CoupleSem L mem pacing Tc Td hjT).fold g init ok s).wf = s.wf := rfl

theorem co_fold_monotone_rr {α σ : Type} [DecidableEq α]
    {ord : StrOrd} {ret : Retries} (vo : ValueOrder σ) (g : σ → α → σ)
    (init : σ) (ok : FoldOk ord ret g) (hinfl : ∀ s x, vo.le s (g s x))
    (s : (CoupleSem L mem pacing Tc Td hjT).Stream ℓ α ord ret) :
    ((CoupleSem L mem pacing Tc Td hjT).fold_monotone vo g init ok hinfl s).rr
      = (Values L mem).fold_monotone vo g init ok hinfl s.rr := rfl

theorem co_fold_monotone_sr {α σ : Type} [DecidableEq α]
    {ord : StrOrd} {ret : Retries} (vo : ValueOrder σ) (g : σ → α → σ)
    (init : σ) (ok : FoldOk ord ret g) (hinfl : ∀ s x, vo.le s (g s x))
    (s : (CoupleSem L mem pacing Tc Td hjT).Stream ℓ α ord ret) :
    ((CoupleSem L mem pacing Tc Td hjT).fold_monotone vo g init ok hinfl s).sr
      = (SchedSem L mem pacing).fold_monotone (ℓ := ℓ) (ord := ord)
          (ret := ret) vo g init ok hinfl s.sr := rfl

theorem co_fold_monotone_wf {α σ : Type} [DecidableEq α]
    {ord : StrOrd} {ret : Retries} (vo : ValueOrder σ) (g : σ → α → σ)
    (init : σ) (ok : FoldOk ord ret g) (hinfl : ∀ s x, vo.le s (g s x))
    (s : (CoupleSem L mem pacing Tc Td hjT).Stream ℓ α ord ret) :
    ((CoupleSem L mem pacing Tc Td hjT).fold_monotone vo g init ok hinfl s).wf
      = s.wf := rfl

theorem co_snapshot_rr {α σ : Type} [DecidableEq α] {ord : StrOrd}
    {ret : Retries} {b : SingBound σ}
    (s : (CoupleSem L mem pacing Tc Td hjT).Singleton ℓ α σ ord ret b)
    (cutd : (CoupleSem L mem pacing Tc Td hjT).SnapDec (mem ℓ) α ord) :
    ((CoupleSem L mem pacing Tc Td hjT).snapshot s cutd).rr
      = (Values L mem).snapshot (ret := ret) s.rr
          (snapDerive ord ret (pacing ℓ) Td s.sr) := rfl

theorem co_snapshot_sr {α σ : Type} [DecidableEq α] {ord : StrOrd}
    {ret : Retries} {b : SingBound σ}
    (s : (CoupleSem L mem pacing Tc Td hjT).Singleton ℓ α σ ord ret b)
    (cutd : (CoupleSem L mem pacing Tc Td hjT).SnapDec (mem ℓ) α ord) :
    ((CoupleSem L mem pacing Tc Td hjT).snapshot s cutd).sr
      = (SchedSem L mem pacing).snapshot (ℓ := ℓ) (ord := ord)
          (ret := ret) (b := b) s.sr () := rfl

theorem co_snapshot_wf {α σ : Type} [DecidableEq α] {ord : StrOrd}
    {ret : Retries} {b : SingBound σ}
    (s : (CoupleSem L mem pacing Tc Td hjT).Singleton ℓ α σ ord ret b)
    (cutd : (CoupleSem L mem pacing Tc Td hjT).SnapDec (mem ℓ) α ord) :
    ((CoupleSem L mem pacing Tc Td hjT).snapshot s cutd).wf = s.wf := rfl

theorem co_batch_rr {α : Type} [DecidableEq α]
    (s : (CoupleSem L mem pacing Tc Td hjT).Stream ℓ α .noOrder .exactlyOnce)
    (cutd : (CoupleSem L mem pacing Tc Td hjT).BatchDec (mem ℓ) α) :
    ((CoupleSem L mem pacing Tc Td hjT).batch s cutd).rr
      = (Values L mem).batch s.rr (batchDerive (pacing ℓ) Td s.sr)
    := rfl

theorem co_batch_sr {α : Type} [DecidableEq α]
    (s : (CoupleSem L mem pacing Tc Td hjT).Stream ℓ α .noOrder .exactlyOnce)
    (cutd : (CoupleSem L mem pacing Tc Td hjT).BatchDec (mem ℓ) α) :
    ((CoupleSem L mem pacing Tc Td hjT).batch s cutd).sr
      = (SchedSem L mem pacing).batch (ℓ := ℓ) s.sr () := rfl

theorem co_batch_wf {α : Type} [DecidableEq α]
    (s : (CoupleSem L mem pacing Tc Td hjT).Stream ℓ α .noOrder .exactlyOnce)
    (cutd : (CoupleSem L mem pacing Tc Td hjT).BatchDec (mem ℓ) α) :
    ((CoupleSem L mem pacing Tc Td hjT).batch s cutd).wf = s.wf := rfl

theorem co_batch_ordered_rr {α : Type} [DecidableEq α]
    (s : (CoupleSem L mem pacing Tc Td hjT).Stream ℓ α .totalOrder
      .exactlyOnce)
    (cutd : (CoupleSem L mem pacing Tc Td hjT).OrdBatchDec (mem ℓ)) :
    ((CoupleSem L mem pacing Tc Td hjT).batch_ordered s cutd).rr
      = (Values L mem).batch_ordered s.rr
          (batchOrdDerive (pacing ℓ) Td s.sr) := rfl

theorem co_batch_ordered_sr {α : Type} [DecidableEq α]
    (s : (CoupleSem L mem pacing Tc Td hjT).Stream ℓ α .totalOrder
      .exactlyOnce)
    (cutd : (CoupleSem L mem pacing Tc Td hjT).OrdBatchDec (mem ℓ)) :
    ((CoupleSem L mem pacing Tc Td hjT).batch_ordered s cutd).sr
      = (SchedSem L mem pacing).batch_ordered (ℓ := ℓ) s.sr () := rfl

theorem co_batch_ordered_wf {α : Type} [DecidableEq α]
    (s : (CoupleSem L mem pacing Tc Td hjT).Stream ℓ α .totalOrder
      .exactlyOnce)
    (cutd : (CoupleSem L mem pacing Tc Td hjT).OrdBatchDec (mem ℓ)) :
    ((CoupleSem L mem pacing Tc Td hjT).batch_ordered s cutd).wf = s.wf := rfl

theorem co_mapTick_rr {α β : Type}
    (s : (CoupleSem L mem pacing Tc Td hjT).Ticked ℓ α)
    (f : Fin (mem ℓ) → α → β) :
    ((CoupleSem L mem pacing Tc Td hjT).mapTick (ℓ := ℓ) s f).rr
      = (Values L mem).mapTick (ℓ := ℓ) s.rr f := rfl

theorem co_mapTick_sr {α β : Type}
    (s : (CoupleSem L mem pacing Tc Td hjT).Ticked ℓ α)
    (f : Fin (mem ℓ) → α → β) :
    ((CoupleSem L mem pacing Tc Td hjT).mapTick (ℓ := ℓ) s f).sr
      = (SchedSem L mem pacing).mapTick (ℓ := ℓ) s.sr f := rfl

theorem co_mapTick_wf {α β : Type}
    (s : (CoupleSem L mem pacing Tc Td hjT).Ticked ℓ α)
    (f : Fin (mem ℓ) → α → β) :
    ((CoupleSem L mem pacing Tc Td hjT).mapTick (ℓ := ℓ) s f).wf
      = s.wf := rfl

theorem co_zipTick_rr {α β : Type}
    (a : (CoupleSem L mem pacing Tc Td hjT).Ticked ℓ α)
    (b : (CoupleSem L mem pacing Tc Td hjT).Ticked ℓ β)
    :
    ((CoupleSem L mem pacing Tc Td hjT).zipTick (ℓ := ℓ) a b).rr
      = (Values L mem).zipTick (ℓ := ℓ) a.rr b.rr := rfl

theorem co_zipTick_sr {α β : Type}
    (a : (CoupleSem L mem pacing Tc Td hjT).Ticked ℓ α)
    (b : (CoupleSem L mem pacing Tc Td hjT).Ticked ℓ β) :
    ((CoupleSem L mem pacing Tc Td hjT).zipTick (ℓ := ℓ) a b).sr
      = (SchedSem L mem pacing).zipTick (ℓ := ℓ) a.sr b.sr := rfl

theorem co_zipTick_wf {α β : Type}
    (a : (CoupleSem L mem pacing Tc Td hjT).Ticked ℓ α)
    (b : (CoupleSem L mem pacing Tc Td hjT).Ticked ℓ β) :
    ((CoupleSem L mem pacing Tc Td hjT).zipTick (ℓ := ℓ) a b).wf
      = (a.wf ∧ b.wf) := rfl

/-! ### In-tick operators (D60): legwise projections, and the
conditional denotation agreement of plain-typed results

Stream-typed in-tick operators project to the two legs by `rfl` (they
act legwise and transport the coupling). Plain-typed results (`count`,
`fold`, `first`) are computed from the machine leg, so their `sr`
projection is `rfl` and their denotation agreement is CONDITIONAL on
the pair's `wf` — the proof the List-vs-quotient split forces. -/

section InTick
variable {α β σ : Type} [DecidableEq α] [DecidableEq β] {ord : StrOrd}

theorem co_bmap_rr (b : (CoupleSem L mem pacing Tc Td hjT).BoundedStream
    α ord .exactlyOnce) (f : α → β) :
    ((CoupleSem L mem pacing Tc Td hjT).bmap b f).rr
      = (Values L mem).bmap b.rr f := rfl
theorem co_bmap_sr (b : (CoupleSem L mem pacing Tc Td hjT).BoundedStream
    α ord .exactlyOnce) (f : α → β) :
    ((CoupleSem L mem pacing Tc Td hjT).bmap b f).sr
      = (SchedSem L mem pacing).bmap (ord := ord) b.sr f := rfl
theorem co_bmap_wf (b : (CoupleSem L mem pacing Tc Td hjT).BoundedStream
    α ord .exactlyOnce) (f : α → β) :
    ((CoupleSem L mem pacing Tc Td hjT).bmap b f).wf = b.wf := rfl

theorem co_bfilterMap_rr (b : (CoupleSem L mem pacing Tc Td hjT).BoundedStream
    α ord .exactlyOnce) (f : α → Option β) :
    ((CoupleSem L mem pacing Tc Td hjT).bfilterMap b f).rr
      = (Values L mem).bfilterMap b.rr f := rfl
theorem co_bfilterMap_sr (b : (CoupleSem L mem pacing Tc Td hjT).BoundedStream
    α ord .exactlyOnce) (f : α → Option β) :
    ((CoupleSem L mem pacing Tc Td hjT).bfilterMap b f).sr
      = (SchedSem L mem pacing).bfilterMap (ord := ord) b.sr f := rfl
theorem co_bfilterMap_wf (b : (CoupleSem L mem pacing Tc Td hjT).BoundedStream
    α ord .exactlyOnce) (f : α → Option β) :
    ((CoupleSem L mem pacing Tc Td hjT).bfilterMap b f).wf = b.wf := rfl

theorem co_bflatMapOrdered_rr
    (l : (CoupleSem L mem pacing Tc Td hjT).BoundedStream α .totalOrder .exactlyOnce)
    (f : α → List β) :
    ((CoupleSem L mem pacing Tc Td hjT).bflatMapOrdered l f).rr
      = (Values L mem).bflatMapOrdered l.rr f := rfl
theorem co_bflatMapOrdered_sr
    (l : (CoupleSem L mem pacing Tc Td hjT).BoundedStream α .totalOrder .exactlyOnce)
    (f : α → List β) :
    ((CoupleSem L mem pacing Tc Td hjT).bflatMapOrdered l f).sr
      = (SchedSem L mem pacing).bflatMapOrdered l.sr f := rfl
theorem co_bflatMapOrdered_wf
    (l : (CoupleSem L mem pacing Tc Td hjT).BoundedStream α .totalOrder .exactlyOnce)
    (f : α → List β) :
    ((CoupleSem L mem pacing Tc Td hjT).bflatMapOrdered l f).wf = l.wf := rfl

theorem co_bflatMapUnordered_rr
    (b : (CoupleSem L mem pacing Tc Td hjT).BoundedStream α ord .exactlyOnce)
    (f : α → (CoupleSem L mem pacing Tc Td hjT).BoundedStream β .noOrder .exactlyOnce) :
    ((CoupleSem L mem pacing Tc Td hjT).bflatMapUnordered b f).rr
      = (Values L mem).bflatMapUnordered b.rr (fun a => (f a).rr) := rfl
theorem co_bflatMapUnordered_sr
    (b : (CoupleSem L mem pacing Tc Td hjT).BoundedStream α ord .exactlyOnce)
    (f : α → (CoupleSem L mem pacing Tc Td hjT).BoundedStream β .noOrder .exactlyOnce) :
    ((CoupleSem L mem pacing Tc Td hjT).bflatMapUnordered b f).sr
      = (SchedSem L mem pacing).bflatMapUnordered (ord := ord) b.sr (fun a => (f a).sr) := rfl
theorem co_bflatMapUnordered_wf
    (b : (CoupleSem L mem pacing Tc Td hjT).BoundedStream α ord .exactlyOnce)
    (f : α → (CoupleSem L mem pacing Tc Td hjT).BoundedStream β .noOrder .exactlyOnce) :
    ((CoupleSem L mem pacing Tc Td hjT).bflatMapUnordered b f).wf
      = (b.wf ∧ ∀ a ∈ b.sr, (f a).wf) := rfl

theorem co_bofList_rr (l : List β) :
    ((CoupleSem L mem pacing Tc Td hjT).bofList l).rr
      = (Values L mem).bofList l := rfl
theorem co_bofList_sr (l : List β) :
    ((CoupleSem L mem pacing Tc Td hjT).bofList l).sr
      = (SchedSem L mem pacing).bofList l := rfl
theorem co_bofList_wf (l : List β) :
    ((CoupleSem L mem pacing Tc Td hjT).bofList l).wf = True := rfl

theorem co_benumerate_rr
    (l : (CoupleSem L mem pacing Tc Td hjT).BoundedStream α .totalOrder .exactlyOnce) :
    ((CoupleSem L mem pacing Tc Td hjT).benumerate l).rr
      = (Values L mem).benumerate l.rr := rfl
theorem co_benumerate_sr
    (l : (CoupleSem L mem pacing Tc Td hjT).BoundedStream α .totalOrder .exactlyOnce) :
    ((CoupleSem L mem pacing Tc Td hjT).benumerate l).sr
      = (SchedSem L mem pacing).benumerate l.sr := rfl
theorem co_benumerate_wf
    (l : (CoupleSem L mem pacing Tc Td hjT).BoundedStream α .totalOrder .exactlyOnce) :
    ((CoupleSem L mem pacing Tc Td hjT).benumerate l).wf = l.wf := rfl

theorem co_bcrossSingleton_rr [DecidableEq σ]
    (b : (CoupleSem L mem pacing Tc Td hjT).BoundedStream α ord .exactlyOnce)
    (s : (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton σ) :
    ((CoupleSem L mem pacing Tc Td hjT).bcrossSingleton b s).rr
      = (Values L mem).bcrossSingleton b.rr s.rr := rfl
theorem co_bcrossSingleton_sr [DecidableEq σ]
    (b : (CoupleSem L mem pacing Tc Td hjT).BoundedStream α ord .exactlyOnce)
    (s : (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton σ) :
    ((CoupleSem L mem pacing Tc Td hjT).bcrossSingleton b s).sr
      = (SchedSem L mem pacing).bcrossSingleton (ord := ord) b.sr s.sr := rfl
theorem co_bcrossSingleton_wf [DecidableEq σ]
    (b : (CoupleSem L mem pacing Tc Td hjT).BoundedStream α ord .exactlyOnce)
    (s : (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton σ) :
    ((CoupleSem L mem pacing Tc Td hjT).bcrossSingleton b s).wf = (b.wf ∧ s.wf) := rfl

theorem co_bchain_rr (a b : (CoupleSem L mem pacing Tc Td hjT).BoundedStream α ord .exactlyOnce) :
    ((CoupleSem L mem pacing Tc Td hjT).bchain a b).rr
      = (Values L mem).bchain a.rr b.rr := rfl
theorem co_bchain_sr (a b : (CoupleSem L mem pacing Tc Td hjT).BoundedStream α ord .exactlyOnce) :
    ((CoupleSem L mem pacing Tc Td hjT).bchain a b).sr
      = (SchedSem L mem pacing).bchain (ord := ord) a.sr b.sr := rfl
theorem co_bchain_wf (a b : (CoupleSem L mem pacing Tc Td hjT).BoundedStream α ord .exactlyOnce) :
    ((CoupleSem L mem pacing Tc Td hjT).bchain a b).wf = (a.wf ∧ b.wf) := rfl

theorem co_bweakenOrder_rr (b : (CoupleSem L mem pacing Tc Td hjT).BoundedStream α ord .exactlyOnce) :
    ((CoupleSem L mem pacing Tc Td hjT).bweakenOrder b).rr
      = (Values L mem).bweakenOrder b.rr := rfl
theorem co_bweakenOrder_sr (b : (CoupleSem L mem pacing Tc Td hjT).BoundedStream α ord .exactlyOnce) :
    ((CoupleSem L mem pacing Tc Td hjT).bweakenOrder b).sr
      = (SchedSem L mem pacing).bweakenOrder (ord := ord) b.sr := rfl
theorem co_bweakenOrder_wf (b : (CoupleSem L mem pacing Tc Td hjT).BoundedStream α ord .exactlyOnce) :
    ((CoupleSem L mem pacing Tc Td hjT).bweakenOrder b).wf = b.wf := rfl

theorem co_bfilter_rr (b : (CoupleSem L mem pacing Tc Td hjT).BoundedStream α ord .exactlyOnce) (p : α → Bool) :
    ((CoupleSem L mem pacing Tc Td hjT).bfilter b p).rr = (Values L mem).bfilter b.rr p := rfl
theorem co_bfilter_sr (b : (CoupleSem L mem pacing Tc Td hjT).BoundedStream α ord .exactlyOnce) (p : α → Bool) :
    ((CoupleSem L mem pacing Tc Td hjT).bfilter b p).sr = (SchedSem L mem pacing).bfilter (ord := ord) b.sr p := rfl
theorem co_bfilter_wf (b : (CoupleSem L mem pacing Tc Td hjT).BoundedStream α ord .exactlyOnce) (p : α → Bool) :
    ((CoupleSem L mem pacing Tc Td hjT).bfilter b p).wf = b.wf := rfl

theorem co_bkeyedFold_rr {K V A : Type} [DecidableEq K] [DecidableEq V] [DecidableEq A]
    (g : A → V → A) (init : A) (ok : FoldOk ord .exactlyOnce g)
    (b : (CoupleSem L mem pacing Tc Td hjT).BoundedStream (K × V) ord .exactlyOnce) :
    ((CoupleSem L mem pacing Tc Td hjT).bkeyedFold g init ok b).rr = (Values L mem).bkeyedFold g init ok b.rr := rfl
theorem co_bkeyedFold_sr {K V A : Type} [DecidableEq K] [DecidableEq V] [DecidableEq A]
    (g : A → V → A) (init : A) (ok : FoldOk ord .exactlyOnce g)
    (b : (CoupleSem L mem pacing Tc Td hjT).BoundedStream (K × V) ord .exactlyOnce) :
    ((CoupleSem L mem pacing Tc Td hjT).bkeyedFold g init ok b).sr = (SchedSem L mem pacing).bkeyedFold (ord := ord) g init ok b.sr := rfl
theorem co_bkeyedFold_wf {K V A : Type} [DecidableEq K] [DecidableEq V] [DecidableEq A]
    (g : A → V → A) (init : A) (ok : FoldOk ord .exactlyOnce g)
    (b : (CoupleSem L mem pacing Tc Td hjT).BoundedStream (K × V) ord .exactlyOnce) :
    ((CoupleSem L mem pacing Tc Td hjT).bkeyedFold g init ok b).wf = b.wf := rfl

theorem co_bkeys_rr {K A : Type} [DecidableEq K] [DecidableEq A]
    (e : (CoupleSem L mem pacing Tc Td hjT).BoundedStream (K × A) .noOrder .exactlyOnce) :
    ((CoupleSem L mem pacing Tc Td hjT).bkeys e).rr = (Values L mem).bkeys e.rr := rfl
theorem co_bkeys_sr {K A : Type} [DecidableEq K] [DecidableEq A]
    (e : (CoupleSem L mem pacing Tc Td hjT).BoundedStream (K × A) .noOrder .exactlyOnce) :
    ((CoupleSem L mem pacing Tc Td hjT).bkeys e).sr = (SchedSem L mem pacing).bkeys e.sr := rfl
theorem co_bkeys_wf {K A : Type} [DecidableEq K] [DecidableEq A]
    (e : (CoupleSem L mem pacing Tc Td hjT).BoundedStream (K × A) .noOrder .exactlyOnce) :
    ((CoupleSem L mem pacing Tc Td hjT).bkeys e).wf = e.wf := rfl

theorem co_bjoin_rr {K V W : Type} [DecidableEq K] [DecidableEq V] [DecidableEq W] {ord' : StrOrd}
    (a : (CoupleSem L mem pacing Tc Td hjT).BoundedStream (K × V) ord .exactlyOnce)
    (b : (CoupleSem L mem pacing Tc Td hjT).BoundedStream (K × W) ord' .exactlyOnce) :
    ((CoupleSem L mem pacing Tc Td hjT).bjoin a b).rr = (Values L mem).bjoin a.rr b.rr := rfl
theorem co_bjoin_sr {K V W : Type} [DecidableEq K] [DecidableEq V] [DecidableEq W] {ord' : StrOrd}
    (a : (CoupleSem L mem pacing Tc Td hjT).BoundedStream (K × V) ord .exactlyOnce)
    (b : (CoupleSem L mem pacing Tc Td hjT).BoundedStream (K × W) ord' .exactlyOnce) :
    ((CoupleSem L mem pacing Tc Td hjT).bjoin a b).sr = (SchedSem L mem pacing).bjoin (ord := ord) (ord' := ord') a.sr b.sr := rfl
theorem co_bjoin_wf {K V W : Type} [DecidableEq K] [DecidableEq V] [DecidableEq W] {ord' : StrOrd}
    (a : (CoupleSem L mem pacing Tc Td hjT).BoundedStream (K × V) ord .exactlyOnce)
    (b : (CoupleSem L mem pacing Tc Td hjT).BoundedStream (K × W) ord' .exactlyOnce) :
    ((CoupleSem L mem pacing Tc Td hjT).bjoin a b).wf = (a.wf ∧ b.wf) := rfl

theorem co_bantiJoin_rr {K V : Type} [DecidableEq K] [DecidableEq V] {ord' : StrOrd}
    (a : (CoupleSem L mem pacing Tc Td hjT).BoundedStream (K × V) ord .exactlyOnce)
    (ks : (CoupleSem L mem pacing Tc Td hjT).BoundedStream K ord' .exactlyOnce) :
    ((CoupleSem L mem pacing Tc Td hjT).bantiJoin a ks).rr = (Values L mem).bantiJoin a.rr ks.rr := rfl
theorem co_bantiJoin_sr {K V : Type} [DecidableEq K] [DecidableEq V] {ord' : StrOrd}
    (a : (CoupleSem L mem pacing Tc Td hjT).BoundedStream (K × V) ord .exactlyOnce)
    (ks : (CoupleSem L mem pacing Tc Td hjT).BoundedStream K ord' .exactlyOnce) :
    ((CoupleSem L mem pacing Tc Td hjT).bantiJoin a ks).sr = (SchedSem L mem pacing).bantiJoin (ord := ord) (ord' := ord') a.sr ks.sr := rfl
theorem co_bantiJoin_wf {K V : Type} [DecidableEq K] [DecidableEq V] {ord' : StrOrd}
    (a : (CoupleSem L mem pacing Tc Td hjT).BoundedStream (K × V) ord .exactlyOnce)
    (ks : (CoupleSem L mem pacing Tc Td hjT).BoundedStream K ord' .exactlyOnce) :
    ((CoupleSem L mem pacing Tc Td hjT).bantiJoin a ks).wf = (a.wf ∧ ks.wf) := rfl

theorem co_bfilterNotIn_rr {ord' : StrOrd}
    (a : (CoupleSem L mem pacing Tc Td hjT).BoundedStream α ord .exactlyOnce) (o : (CoupleSem L mem pacing Tc Td hjT).BoundedStream α ord' .exactlyOnce) :
    ((CoupleSem L mem pacing Tc Td hjT).bfilterNotIn a o).rr = (Values L mem).bfilterNotIn a.rr o.rr := rfl
theorem co_bfilterNotIn_sr {ord' : StrOrd}
    (a : (CoupleSem L mem pacing Tc Td hjT).BoundedStream α ord .exactlyOnce) (o : (CoupleSem L mem pacing Tc Td hjT).BoundedStream α ord' .exactlyOnce) :
    ((CoupleSem L mem pacing Tc Td hjT).bfilterNotIn a o).sr = (SchedSem L mem pacing).bfilterNotIn (ord := ord) (ord' := ord') a.sr o.sr := rfl
theorem co_bfilterNotIn_wf {ord' : StrOrd}
    (a : (CoupleSem L mem pacing Tc Td hjT).BoundedStream α ord .exactlyOnce) (o : (CoupleSem L mem pacing Tc Td hjT).BoundedStream α ord' .exactlyOnce) :
    ((CoupleSem L mem pacing Tc Td hjT).bfilterNotIn a o).wf = (a.wf ∧ o.wf) := rfl

theorem co_bmax_rr [LinearOrder α] (b : (CoupleSem L mem pacing Tc Td hjT).BoundedStream α ord .exactlyOnce) :
    ((CoupleSem L mem pacing Tc Td hjT).bmax b).rr = (Values L mem).bmax b.rr := rfl
theorem co_bmax_sr [LinearOrder α] (b : (CoupleSem L mem pacing Tc Td hjT).BoundedStream α ord .exactlyOnce) :
    ((CoupleSem L mem pacing Tc Td hjT).bmax b).sr = (SchedSem L mem pacing).bmax (ord := ord) b.sr := rfl
theorem co_bmax_wf [LinearOrder α] (b : (CoupleSem L mem pacing Tc Td hjT).BoundedStream α ord .exactlyOnce) :
    ((CoupleSem L mem pacing Tc Td hjT).bmax b).wf = b.wf := rfl

/-! A program-level `if` over static parameters (Rust's construction-time
`if max == min { … } else { … }` in quorum.rs) selects between two
in-tick values; the legs commute with the choice. The pushed `ite` is
spelled at the LEG'S carrier type (not the projection's raw codomain):
simp's syntactic matching of the surrounding `Eq` needs the types to
coincide at reducible transparency. -/
theorem co_ite_bstream_sr {ret : Retries} (c : Prop) [Decidable c]
    (a b : (CoupleSem L mem pacing Tc Td hjT).BoundedStream α ord ret) :
    (ite c a b).sr
      = @ite ((SchedSem L mem pacing).BoundedStream α ord ret) c _ a.sr b.sr := by
  cases ‹Decidable c› <;> rfl
theorem co_ite_bstream_rr {ret : Retries} (c : Prop) [Decidable c]
    (a b : (CoupleSem L mem pacing Tc Td hjT).BoundedStream α ord ret) :
    (ite c a b).rr
      = @ite ((Values L mem).BoundedStream α ord ret) c _ a.rr b.rr := by
  cases ‹Decidable c› <;> rfl
theorem co_ite_bstream_wf {ret : Retries} (c : Prop) [Decidable c]
    (a b : (CoupleSem L mem pacing Tc Td hjT).BoundedStream α ord ret) :
    (ite c a b).wf = ite c a.wf b.wf := by
  cases ‹Decidable c› <;> rfl
theorem co_ite_bsing_sr {σ : Type} (c : Prop) [Decidable c]
    (a b : (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton σ) :
    (ite c a b).sr
      = @ite ((SchedSem L mem pacing).BoundedSingleton σ) c _ a.sr b.sr := by
  cases ‹Decidable c› <;> rfl
theorem co_ite_bsing_rr {σ : Type} (c : Prop) [Decidable c]
    (a b : (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton σ) :
    (ite c a b).rr
      = @ite ((Values L mem).BoundedSingleton σ) c _ a.rr b.rr := by
  cases ‹Decidable c› <;> rfl
theorem co_ite_bsing_wf {σ : Type} (c : Prop) [Decidable c]
    (a b : (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton σ) :
    (ite c a b).wf = ite c a.wf b.wf := by
  cases ‹Decidable c› <;> rfl
/-- A chosen tuple, projected: the projections commute with the choice. -/
theorem fst_ite {α β : Type} (c : Prop) [Decidable c] (a b : α × β) :
    (ite c a b).1 = ite c a.1 b.1 := apply_ite _ c a b
theorem snd_ite {α β : Type} (c : Prop) [Decidable c] (a b : α × β) :
    (ite c a b).2 = ite c a.2 b.2 := apply_ite _ c a b

theorem co_bfilterIf_rr {ret : Retries}
    (b : (CoupleSem L mem pacing Tc Td hjT).BoundedStream α ord ret)
    (flag : (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton Bool) :
    ((CoupleSem L mem pacing Tc Td hjT).bfilterIf b flag).rr
      = (Values L mem).bfilterIf b.rr flag.rr := rfl
theorem co_bfilterIf_sr {ret : Retries}
    (b : (CoupleSem L mem pacing Tc Td hjT).BoundedStream α ord ret)
    (flag : (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton Bool) :
    ((CoupleSem L mem pacing Tc Td hjT).bfilterIf b flag).sr
      = (SchedSem L mem pacing).bfilterIf (ord := ord) (ret := ret) b.sr flag.sr := rfl
theorem co_bfilterIf_wf {ret : Retries}
    (b : (CoupleSem L mem pacing Tc Td hjT).BoundedStream α ord ret)
    (flag : (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton Bool) :
    ((CoupleSem L mem pacing Tc Td hjT).bfilterIf b flag).wf = (b.wf ∧ flag.wf) := rfl

/-- Plain-typed results keep both legs too (`BoundedSingleton` at the
corner is a coupled pair): `count`/`fold`/`first` project legwise, and
their coupling (`cpl`) is the per-tick agreement the grade constraint
makes provable (`count` at AtLeastOnce would be false). -/
theorem co_bcount_rr (b : (CoupleSem L mem pacing Tc Td hjT).BoundedStream α ord .exactlyOnce) :
    ((CoupleSem L mem pacing Tc Td hjT).bcount b).rr
      = (Values L mem).bcount b.rr := rfl
theorem co_bcount_sr (b : (CoupleSem L mem pacing Tc Td hjT).BoundedStream α ord .exactlyOnce) :
    ((CoupleSem L mem pacing Tc Td hjT).bcount b).sr
      = (SchedSem L mem pacing).bcount (ord := ord) b.sr := rfl
theorem co_bcount_wf (b : (CoupleSem L mem pacing Tc Td hjT).BoundedStream α ord .exactlyOnce) :
    ((CoupleSem L mem pacing Tc Td hjT).bcount b).wf = b.wf := rfl

theorem co_bfold_rr {ret : Retries} (g : σ → α → σ) (init : σ)
    (ok : FoldOk ord ret g)
    (b : (CoupleSem L mem pacing Tc Td hjT).BoundedStream α ord ret) :
    ((CoupleSem L mem pacing Tc Td hjT).bfold g init ok b).rr
      = (Values L mem).bfold g init ok b.rr := rfl
theorem co_bfold_sr {ret : Retries} (g : σ → α → σ) (init : σ)
    (ok : FoldOk ord ret g)
    (b : (CoupleSem L mem pacing Tc Td hjT).BoundedStream α ord ret) :
    ((CoupleSem L mem pacing Tc Td hjT).bfold g init ok b).sr
      = (SchedSem L mem pacing).bfold g init ok b.sr := rfl
theorem co_bfold_wf {ret : Retries} (g : σ → α → σ) (init : σ)
    (ok : FoldOk ord ret g)
    (b : (CoupleSem L mem pacing Tc Td hjT).BoundedStream α ord ret) :
    ((CoupleSem L mem pacing Tc Td hjT).bfold g init ok b).wf
      = (b.wf ∧ ret = .exactlyOnce) := rfl

theorem co_bfirst_rr
    (l : (CoupleSem L mem pacing Tc Td hjT).BoundedStream α .totalOrder .exactlyOnce) :
    ((CoupleSem L mem pacing Tc Td hjT).bfirst l).rr
      = (Values L mem).bfirst l.rr := rfl
theorem co_bfirst_sr
    (l : (CoupleSem L mem pacing Tc Td hjT).BoundedStream α .totalOrder .exactlyOnce) :
    ((CoupleSem L mem pacing Tc Td hjT).bfirst l).sr
      = (SchedSem L mem pacing).bfirst l.sr := rfl
theorem co_bfirst_wf
    (l : (CoupleSem L mem pacing Tc Td hjT).BoundedStream α .totalOrder .exactlyOnce) :
    ((CoupleSem L mem pacing Tc Td hjT).bfirst l).wf = l.wf := rfl

-- the singleton/optional API: legwise
theorem co_bsPure_rr (v : σ) :
    ((CoupleSem L mem pacing Tc Td hjT).bsPure v).rr = (Values L mem).bsPure v := rfl
theorem co_bsPure_sr (v : σ) :
    ((CoupleSem L mem pacing Tc Td hjT).bsPure v).sr = (SchedSem L mem pacing).bsPure v := rfl
theorem co_bsPure_wf (v : σ) :
    ((CoupleSem L mem pacing Tc Td hjT).bsPure v).wf = True := rfl
theorem co_bsMap_rr {τ : Type} (x : (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton σ) (f : σ → τ) :
    ((CoupleSem L mem pacing Tc Td hjT).bsMap x f).rr = (Values L mem).bsMap x.rr f := rfl
theorem co_bsMap_sr {τ : Type} (x : (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton σ) (f : σ → τ) :
    ((CoupleSem L mem pacing Tc Td hjT).bsMap x f).sr = (SchedSem L mem pacing).bsMap x.sr f := rfl
theorem co_bsMap_wf {τ : Type} (x : (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton σ) (f : σ → τ) :
    ((CoupleSem L mem pacing Tc Td hjT).bsMap x f).wf = x.wf := rfl
theorem co_bsZip_rr {τ : Type} (a : (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton σ)
    (b : (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton τ) :
    ((CoupleSem L mem pacing Tc Td hjT).bsZip a b).rr = (Values L mem).bsZip a.rr b.rr := rfl
theorem co_bsZip_sr {τ : Type} (a : (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton σ)
    (b : (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton τ) :
    ((CoupleSem L mem pacing Tc Td hjT).bsZip a b).sr = (SchedSem L mem pacing).bsZip a.sr b.sr := rfl
theorem co_bsZip_wf {τ : Type} (a : (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton σ)
    (b : (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton τ) :
    ((CoupleSem L mem pacing Tc Td hjT).bsZip a b).wf = (a.wf ∧ b.wf) := rfl
theorem co_boMap_rr {τ : Type} (o : (CoupleSem L mem pacing Tc Td hjT).BoundedOptional σ) (f : σ → τ) :
    ((CoupleSem L mem pacing Tc Td hjT).boMap o f).rr = (Values L mem).boMap o.rr f := rfl
theorem co_boMap_sr {τ : Type} (o : (CoupleSem L mem pacing Tc Td hjT).BoundedOptional σ) (f : σ → τ) :
    ((CoupleSem L mem pacing Tc Td hjT).boMap o f).sr = (SchedSem L mem pacing).boMap o.sr f := rfl
theorem co_boMap_wf {τ : Type} (o : (CoupleSem L mem pacing Tc Td hjT).BoundedOptional σ) (f : σ → τ) :
    ((CoupleSem L mem pacing Tc Td hjT).boMap o f).wf = o.wf := rfl
theorem co_boUnwrapOr_rr (o : (CoupleSem L mem pacing Tc Td hjT).BoundedOptional σ)
    (x : (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton σ) :
    ((CoupleSem L mem pacing Tc Td hjT).boUnwrapOr o x).rr = (Values L mem).boUnwrapOr o.rr x.rr := rfl
theorem co_boUnwrapOr_sr (o : (CoupleSem L mem pacing Tc Td hjT).BoundedOptional σ)
    (x : (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton σ) :
    ((CoupleSem L mem pacing Tc Td hjT).boUnwrapOr o x).sr = (SchedSem L mem pacing).boUnwrapOr o.sr x.sr := rfl
theorem co_boUnwrapOr_wf (o : (CoupleSem L mem pacing Tc Td hjT).BoundedOptional σ)
    (x : (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton σ) :
    ((CoupleSem L mem pacing Tc Td hjT).boUnwrapOr o x).wf = (o.wf ∧ x.wf) := rfl
theorem co_boFilter_rr (o : (CoupleSem L mem pacing Tc Td hjT).BoundedOptional σ) (p : σ → Bool) :
    ((CoupleSem L mem pacing Tc Td hjT).boFilter o p).rr = (Values L mem).boFilter o.rr p := rfl
theorem co_boFilter_sr (o : (CoupleSem L mem pacing Tc Td hjT).BoundedOptional σ) (p : σ → Bool) :
    ((CoupleSem L mem pacing Tc Td hjT).boFilter o p).sr = (SchedSem L mem pacing).boFilter o.sr p := rfl
theorem co_boFilter_wf (o : (CoupleSem L mem pacing Tc Td hjT).BoundedOptional σ) (p : σ → Bool) :
    ((CoupleSem L mem pacing Tc Td hjT).boFilter o p).wf = o.wf := rfl
theorem co_boIsSome_rr (o : (CoupleSem L mem pacing Tc Td hjT).BoundedOptional σ) :
    ((CoupleSem L mem pacing Tc Td hjT).boIsSome o).rr = (Values L mem).boIsSome o.rr := rfl
theorem co_boIsSome_sr (o : (CoupleSem L mem pacing Tc Td hjT).BoundedOptional σ) :
    ((CoupleSem L mem pacing Tc Td hjT).boIsSome o).sr = (SchedSem L mem pacing).boIsSome o.sr := rfl
theorem co_boIsSome_wf (o : (CoupleSem L mem pacing Tc Td hjT).BoundedOptional σ) :
    ((CoupleSem L mem pacing Tc Td hjT).boIsSome o).wf = o.wf := rfl

/-! ### The shaped tick former at the corner: the instance-typed facade

The corner's `tick_scan` is `CoTick.mk` of the two legs' formers
(`CoTick.scan`, Couple.lean). The simp walk, however, unifies at
`instances` transparency and cannot see `CoBSing σ ≡ (CoupleSem …
).BoundedSingleton σ` through the semireducible instance (unfolding it
is the whole-program-whnf hazard, D56). So every term the walk
produces is spelled through this facade — definitionally the generic
`CoTick` machinery, but DECLARED at the interpretations' own carrier
types, which is what a program's goals carry. All rules are `rfl`. -/

namespace CoTickI
variable {a b : TickShape} {τ : Type} {inst : DecidableEq α} {ret : Retries}
  (sts ins outs : TickShape)

def srLegs (sh : TickShape) (x : TickedOf (CoupleSem L mem pacing Tc Td hjT).Ticked (CoupleSem L mem pacing Tc Td hjT).TickStream ℓ (sh)) : TickedOf (SchedSem L mem pacing).Ticked (SchedSem L mem pacing).TickStream ℓ (sh) := CoTick.srLegs sh x
def rrLegs (sh : TickShape) (x : TickedOf (CoupleSem L mem pacing Tc Td hjT).Ticked (CoupleSem L mem pacing Tc Td hjT).TickStream ℓ (sh)) : TickedOf (Values L mem).Ticked (Values L mem).TickStream ℓ (sh) := CoTick.rrLegs sh x
def Wf (sh : TickShape) (x : TickedOf (CoupleSem L mem pacing Tc Td hjT).Ticked (CoupleSem L mem pacing Tc Td hjT).TickStream ℓ (sh)) : Prop := CoTick.Wf sh x
def mk (sh : TickShape) (m : TickedOf (SchedSem L mem pacing).Ticked (SchedSem L mem pacing).TickStream ℓ (sh)) (v : TickedOf (Values L mem).Ticked (Values L mem).TickStream ℓ (sh)) (wf : Prop)
    (h : wf → ∀ k, k ≤ Tc → CorrTick.Le sh m k v) : TickedOf (CoupleSem L mem pacing Tc Td hjT).Ticked (CoupleSem L mem pacing Tc Td hjT).TickStream ℓ (sh) :=
  CoTick.mk sh m v wf h
def projSR (sh : TickShape) (x : BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sh)) : BoundedOf (SchedSem L mem pacing).BoundedSingleton (SchedSem L mem pacing).BoundedStream (sh) := CoTick.projSR sh x
def projRR (sh : TickShape) (x : BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sh)) : BoundedOf (Values L mem).BoundedSingleton (Values L mem).BoundedStream (sh) := CoTick.projRR sh x
def WfB (sh : TickShape) (x : BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sh)) : Prop := CoTick.WfB sh x
def embedSR (sh : TickShape) (m : BoundedOf (SchedSem L mem pacing).BoundedSingleton (SchedSem L mem pacing).BoundedStream (sh)) : BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sh) := CoTick.embedSR sh m
def embedRR (sh : TickShape) (v : BoundedOf (Values L mem).BoundedSingleton (Values L mem).BoundedStream (sh)) : BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sh) := CoTick.embedRR sh v
def embed2 (sh : TickShape) (m : BoundedOf (SchedSem L mem pacing).BoundedSingleton (SchedSem L mem pacing).BoundedStream (sh)) (v : BoundedOf (Values L mem).BoundedSingleton (Values L mem).BoundedStream (sh)) : BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sh) :=
  CoTick.embed2 sh m v
def coerce (sh : TickShape) (m : BoundedOf (SchedSem L mem pacing).BoundedSingleton (SchedSem L mem pacing).BoundedStream (sh)) : BoundedOf (Values L mem).BoundedSingleton (Values L mem).BoundedStream (sh) := CoTick.coerce sh m
/-- The machine's singleton value, presented at the denotation's type
(definitionally the identity; keeps the walk's terms consistently
typed so the leaf rules unify). -/
abbrev idV (v : (SchedSem L mem pacing).BoundedSingleton σ) : (Values L mem).BoundedSingleton σ := v
def gS (g : Fin (mem ℓ) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (ins) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) × BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (outs)) : Fin (mem ℓ) → BoundedOf (SchedSem L mem pacing).BoundedSingleton (SchedSem L mem pacing).BoundedStream (sts) → BoundedOf (SchedSem L mem pacing).BoundedSingleton (SchedSem L mem pacing).BoundedStream (ins) →
    BoundedOf (SchedSem L mem pacing).BoundedSingleton (SchedSem L mem pacing).BoundedStream (sts) × BoundedOf (SchedSem L mem pacing).BoundedSingleton (SchedSem L mem pacing).BoundedStream (outs) :=
  fun i s inp => @Prod.mk (BoundedOf (SchedSem L mem pacing).BoundedSingleton (SchedSem L mem pacing).BoundedStream (sts))
    (BoundedOf (SchedSem L mem pacing).BoundedSingleton (SchedSem L mem pacing).BoundedStream outs)
    (projSR sts (g i (embedSR sts s) (embedSR ins inp)).1)
    (projSR outs (g i (embedSR sts s) (embedSR ins inp)).2)
def gR (g : Fin (mem ℓ) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (ins) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) × BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (outs)) : Fin (mem ℓ) → BoundedOf (Values L mem).BoundedSingleton (Values L mem).BoundedStream (sts) → BoundedOf (Values L mem).BoundedSingleton (Values L mem).BoundedStream (ins) →
    BoundedOf (Values L mem).BoundedSingleton (Values L mem).BoundedStream (sts) × BoundedOf (Values L mem).BoundedSingleton (Values L mem).BoundedStream (outs) :=
  fun i s inp => @Prod.mk (BoundedOf (Values L mem).BoundedSingleton (Values L mem).BoundedStream (sts))
    (BoundedOf (Values L mem).BoundedSingleton (Values L mem).BoundedStream outs)
    (projRR sts (g i (embedRR sts s) (embedRR ins inp)).1)
    (projRR outs (g i (embedRR sts s) (embedRR ins inp)).2)
def gC (g : Fin (mem ℓ) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (ins) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) × BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (outs)) : Fin (mem ℓ) → BoundedOf (SchedSem L mem pacing).BoundedSingleton (SchedSem L mem pacing).BoundedStream (sts) → BoundedOf (SchedSem L mem pacing).BoundedSingleton (SchedSem L mem pacing).BoundedStream (ins) →
    BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) × BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (outs) :=
  fun i s inp => g i (embed2 sts s (coerce sts s)) (embed2 ins inp (coerce ins inp))
def W1 (g : Fin (mem ℓ) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (ins) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) × BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (outs)) : Prop := ∀ i s inp,
  (gS sts ins outs g i s inp).1 = projSR sts (gC sts ins outs g i s inp).1
    ∧ (gS sts ins outs g i s inp).2 = projSR outs (gC sts ins outs g i s inp).2
def W2 (g : Fin (mem ℓ) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (ins) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) × BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (outs)) : Prop :=
  ∀ i (s : BoundedOf (SchedSem L mem pacing).BoundedSingleton (SchedSem L mem pacing).BoundedStream (sts)) inp,
  (gR sts ins outs g i (coerce sts s) (coerce ins inp)).1 = projRR sts (gC sts ins outs g i s inp).1
    ∧ (gR sts ins outs g i (coerce sts s) (coerce ins inp)).2 = projRR outs (gC sts ins outs g i s inp).2
def W3 (g : Fin (mem ℓ) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (ins) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) × BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (outs)) : Prop := ∀ i s inp,
  WfB sts (gC sts ins outs g i s inp).1 ∧ WfB outs (gC sts ins outs g i s inp).2
def scanWf (x : TickedOf (CoupleSem L mem pacing Tc Td hjT).Ticked (CoupleSem L mem pacing Tc Td hjT).TickStream ℓ (ins)) (g : Fin (mem ℓ) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (ins) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) × BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (outs)) : Prop :=
  Wf ins x ∧ CoTick.allEO sts = true ∧ CoTick.allEO ins = true ∧ CoTick.allEO outs = true
    ∧ W1 sts ins outs g ∧ W2 sts ins outs g ∧ W3 sts ins outs g

/-! The rules. -/

theorem srLegs_pair (x : TickedOf (CoupleSem L mem pacing Tc Td hjT).Ticked (CoupleSem L mem pacing Tc Td hjT).TickStream ℓ (.pair a b)) :
    srLegs (.pair a b) x = (srLegs a x.1, srLegs b x.2) := rfl
theorem srLegs_sing (x : (CoupleSem L mem pacing Tc Td hjT).Ticked ℓ τ) : srLegs (.sing τ) x = x.sr := rfl
theorem srLegs_stream (x : (CoupleSem L mem pacing Tc Td hjT).TickStream ℓ α ord ret) : srLegs (.stream α inst ord ret) x = x.sr := rfl
theorem rrLegs_pair (x : TickedOf (CoupleSem L mem pacing Tc Td hjT).Ticked (CoupleSem L mem pacing Tc Td hjT).TickStream ℓ (.pair a b)) :
    rrLegs (.pair a b) x = (rrLegs a x.1, rrLegs b x.2) := rfl
theorem rrLegs_sing (x : (CoupleSem L mem pacing Tc Td hjT).Ticked ℓ τ) : rrLegs (.sing τ) x = x.rr := rfl
theorem rrLegs_stream (x : (CoupleSem L mem pacing Tc Td hjT).TickStream ℓ α ord ret) : rrLegs (.stream α inst ord ret) x = x.rr := rfl
theorem Wf_pair (x : TickedOf (CoupleSem L mem pacing Tc Td hjT).Ticked (CoupleSem L mem pacing Tc Td hjT).TickStream ℓ (.pair a b)) : Wf (.pair a b) x = (Wf a x.1 ∧ Wf b x.2) := rfl
theorem Wf_sing (x : (CoupleSem L mem pacing Tc Td hjT).Ticked ℓ τ) : Wf (.sing τ) x = x.wf := rfl
theorem Wf_stream (x : (CoupleSem L mem pacing Tc Td hjT).TickStream ℓ α ord ret) : Wf (.stream α inst ord ret) x = x.wf := rfl

theorem projSR_pair (x : BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (.pair a b)) :
    projSR (.pair a b) x = (projSR a x.1, projSR b x.2) := rfl
theorem projSR_sing (x : (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton τ) : projSR (.sing τ) x = x.sr := rfl
theorem projSR_stream (x : (CoupleSem L mem pacing Tc Td hjT).BoundedStream α ord ret) : projSR (.stream α inst ord ret) x = x.sr := rfl
theorem projRR_pair (x : BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (.pair a b)) :
    projRR (.pair a b) x = (projRR a x.1, projRR b x.2) := rfl
theorem projRR_sing (x : (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton τ) : projRR (.sing τ) x = x.rr := rfl
theorem projRR_stream (x : (CoupleSem L mem pacing Tc Td hjT).BoundedStream α ord ret) : projRR (.stream α inst ord ret) x = x.rr := rfl
theorem WfB_pair (x : BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (.pair a b)) : WfB (.pair a b) x = (WfB a x.1 ∧ WfB b x.2) := rfl
theorem WfB_sing (x : (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton τ) : WfB (.sing τ) x = x.wf := rfl
theorem WfB_stream (x : (CoupleSem L mem pacing Tc Td hjT).BoundedStream α ord ret) : WfB (.stream α inst ord ret) x = x.wf := rfl

theorem embedSR_pair (x : BoundedOf (SchedSem L mem pacing).BoundedSingleton (SchedSem L mem pacing).BoundedStream (.pair a b)) :
    embedSR (Td := Td) (hjT := hjT) (.pair a b) x = (embedSR (Td := Td) (hjT := hjT) a x.1, embedSR (Td := Td) (hjT := hjT) b x.2) := rfl
theorem embedSR_sing_sr (v : (SchedSem L mem pacing).BoundedSingleton τ) : (embedSR (Td := Td) (hjT := hjT) (.sing τ) v).sr = v := rfl
theorem embedSR_sing_rr (v : (SchedSem L mem pacing).BoundedSingleton τ) : (embedSR (Td := Td) (hjT := hjT) (.sing τ) v).rr = idV v := rfl
theorem embedSR_sing_wf (v : (SchedSem L mem pacing).BoundedSingleton τ) : (embedSR (Td := Td) (hjT := hjT) (.sing τ) v).wf = False := rfl
theorem embedSR_stream_sr (l : (SchedSem L mem pacing).BoundedStream α ord ret) :
    (embedSR (Td := Td) (hjT := hjT) (.stream α inst ord ret) l).sr = l := rfl
theorem embedSR_stream_wf (l : (SchedSem L mem pacing).BoundedStream α ord ret) :
    (embedSR (Td := Td) (hjT := hjT) (.stream α inst ord ret) l).wf = False := rfl
theorem embedRR_pair (x : BoundedOf (Values L mem).BoundedSingleton (Values L mem).BoundedStream (.pair a b)) :
    embedRR (pacing := pacing) (Td := Td) (hjT := hjT) (.pair a b) x = (embedRR (pacing := pacing) (Td := Td) (hjT := hjT) a x.1, embedRR (pacing := pacing) (Td := Td) (hjT := hjT) b x.2) := rfl
theorem embedRR_sing_sr (v : (Values L mem).BoundedSingleton τ) : (embedRR (pacing := pacing) (Td := Td) (hjT := hjT) (.sing τ) v).sr = v := rfl
theorem embedRR_sing_rr (v : (Values L mem).BoundedSingleton τ) : (embedRR (pacing := pacing) (Td := Td) (hjT := hjT) (.sing τ) v).rr = v := rfl
theorem embedRR_sing_wf (v : (Values L mem).BoundedSingleton τ) : (embedRR (pacing := pacing) (Td := Td) (hjT := hjT) (.sing τ) v).wf = False := rfl
theorem embedRR_stream_rr (p : (Values L mem).BoundedStream α ord ret) :
    (embedRR (pacing := pacing) (Td := Td) (hjT := hjT) (.stream α inst ord ret) p).rr = p := rfl
theorem embedRR_stream_wf (p : (Values L mem).BoundedStream α ord ret) :
    (embedRR (pacing := pacing) (Td := Td) (hjT := hjT) (.stream α inst ord ret) p).wf = False := rfl
theorem embed2_pair (m : BoundedOf (SchedSem L mem pacing).BoundedSingleton (SchedSem L mem pacing).BoundedStream (.pair a b)) (v : BoundedOf (Values L mem).BoundedSingleton (Values L mem).BoundedStream (.pair a b)) :
    embed2 (Td := Td) (hjT := hjT) (.pair a b) m v = (embed2 (Td := Td) (hjT := hjT) a m.1 v.1, embed2 (Td := Td) (hjT := hjT) b m.2 v.2) := rfl
theorem embed2_sing_sr (m : (SchedSem L mem pacing).BoundedSingleton τ) (v : (Values L mem).BoundedSingleton τ) :
    (embed2 (Td := Td) (hjT := hjT) (.sing τ) m v).sr = m := rfl
theorem embed2_sing_rr (m : (SchedSem L mem pacing).BoundedSingleton τ) (v : (Values L mem).BoundedSingleton τ) :
    (embed2 (Td := Td) (hjT := hjT) (.sing τ) m v).rr = v := rfl
theorem embed2_sing_wf (m : (SchedSem L mem pacing).BoundedSingleton τ) (v : (Values L mem).BoundedSingleton τ) :
    (embed2 (Td := Td) (hjT := hjT) (.sing τ) m v).wf = (m = v) := rfl
theorem embed2_stream_sr (m : (SchedSem L mem pacing).BoundedStream α ord ret) (v : (Values L mem).BoundedStream α ord ret) :
    (embed2 (Td := Td) (hjT := hjT) (.stream α inst ord ret) m v).sr = m := rfl
theorem embed2_stream_rr (m : (SchedSem L mem pacing).BoundedStream α ord ret) (v : (Values L mem).BoundedStream α ord ret) :
    (embed2 (Td := Td) (hjT := hjT) (.stream α inst ord ret) m v).rr = v := rfl
theorem coerce_pair (m : BoundedOf (SchedSem L mem pacing).BoundedSingleton (SchedSem L mem pacing).BoundedStream (.pair a b)) :
    coerce (.pair a b) m = (coerce a m.1, coerce b m.2) := rfl
theorem coerce_sing (v : (SchedSem L mem pacing).BoundedSingleton τ) : coerce (.sing τ) v = idV v := rfl
theorem coupledBatch_coerce (m : (SchedSem L mem pacing).BoundedStream α ord ret) :
    CoupledBatch ord ret m (coerce (.stream α inst ord ret) m) = True :=
  CoTick.coupledBatch_coerce α inst ord ret m
/-- The `W3` leaf at a stream input, closed in one step (the coupled
embedding of a list with its own coercion is well-formed). -/
theorem embed2_stream_coerce_wf (m : (SchedSem L mem pacing).BoundedStream α ord ret) :
    (embed2 (Td := Td) (hjT := hjT) (.stream α inst ord ret) m
      (coerce (.stream α inst ord ret) m)).wf = True :=
  CoTick.coupledBatch_coerce α inst ord ret m
theorem eq_idV_self (m : (SchedSem L mem pacing).BoundedSingleton σ) : (m = idV m) = True :=
  propext ⟨fun _ => trivial, fun _ => rfl⟩
theorem gS_def (g : Fin (mem ℓ) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (ins) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) × BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (outs)) : gS sts ins outs g = fun i s inp =>
    @Prod.mk (BoundedOf (SchedSem L mem pacing).BoundedSingleton (SchedSem L mem pacing).BoundedStream (sts))
      (BoundedOf (SchedSem L mem pacing).BoundedSingleton (SchedSem L mem pacing).BoundedStream outs)
      (projSR sts (g i (embedSR sts s) (embedSR ins inp)).1)
      (projSR outs (g i (embedSR sts s) (embedSR ins inp)).2) := rfl
theorem gR_def (g : Fin (mem ℓ) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (ins) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) × BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (outs)) : gR sts ins outs g = fun i s inp =>
    @Prod.mk (BoundedOf (Values L mem).BoundedSingleton (Values L mem).BoundedStream (sts))
      (BoundedOf (Values L mem).BoundedSingleton (Values L mem).BoundedStream outs)
      (projRR sts (g i (embedRR sts s) (embedRR ins inp)).1)
      (projRR outs (g i (embedRR sts s) (embedRR ins inp)).2) := rfl
theorem gC_def (g : Fin (mem ℓ) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (ins) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) × BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (outs)) : gC sts ins outs g = fun i s inp =>
    g i (embed2 sts s (coerce sts s)) (embed2 ins inp (coerce ins inp)) := rfl
theorem W1_def (g : Fin (mem ℓ) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (ins) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) × BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (outs)) : W1 sts ins outs g = ∀ i s inp,
    (gS sts ins outs g i s inp).1 = projSR sts (gC sts ins outs g i s inp).1
      ∧ (gS sts ins outs g i s inp).2 = projSR outs (gC sts ins outs g i s inp).2 := rfl
theorem W2_def (g : Fin (mem ℓ) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (ins) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) × BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (outs)) : W2 sts ins outs g = ∀ i (s : BoundedOf (SchedSem L mem pacing).BoundedSingleton (SchedSem L mem pacing).BoundedStream (sts)) inp,
    (gR sts ins outs g i (coerce sts s) (coerce ins inp)).1 = projRR sts (gC sts ins outs g i s inp).1
      ∧ (gR sts ins outs g i (coerce sts s) (coerce ins inp)).2 = projRR outs (gC sts ins outs g i s inp).2 := rfl
theorem W3_def (g : Fin (mem ℓ) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (ins) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) × BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (outs)) : W3 sts ins outs g = ∀ i s inp,
    WfB sts (gC sts ins outs g i s inp).1 ∧ WfB outs (gC sts ins outs g i s inp).2 := rfl
theorem scanWf_def (x : TickedOf (CoupleSem L mem pacing Tc Td hjT).Ticked (CoupleSem L mem pacing Tc Td hjT).TickStream ℓ (ins)) (g : Fin (mem ℓ) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (ins) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) × BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (outs)) :
    scanWf sts ins outs x g
      = (Wf ins x ∧ CoTick.allEO sts = true ∧ CoTick.allEO ins = true ∧ CoTick.allEO outs = true
        ∧ W1 sts ins outs g ∧ W2 sts ins outs g ∧ W3 sts ins outs g) := rfl

/-- The coupling, at the facade's types (the proof argument of the
corner's `mk`; its TYPE must be spelled through the facade too, or the
`mk` projection rules cannot unify it). -/
theorem scan_cpl (x : TickedOf (CoupleSem L mem pacing Tc Td hjT).Ticked (CoupleSem L mem pacing Tc Td hjT).TickStream ℓ (ins)) (g : Fin (mem ℓ) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (ins) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) × BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (outs)) (init : SeedOf sts) :
    scanWf sts ins outs x g → ∀ k, k ≤ Tc →
      CorrTick.Le outs ((SchedSem L mem pacing).tick_scan sts ins outs (srLegs ins x) (gS sts ins outs g) init) k
        ((Values L mem).tick_scan sts ins outs (rrLegs ins x) (gR sts ins outs g) init) :=
  CoTick.scan_cpl pacing sts ins outs x g init

end CoTickI

/-! ### The shaped tick former at the corner: leg projections by output
PATH. One rule per (projection path into the output tuple, leaf kind,
leg): the rewrite is keyed on the leg projection of a projected
`tick_scan`, so it fires only where the walk's `.sr`/`.rr`/`.wf`
reaches a former's output — never on a bare `tick_scan` inside a
callee's argument — and its right-hand side has no proof argument
(the `mk`-with-coupling spelling has one, whose type the walk cannot
keep in step with the rewritten legs). Right-nested outputs of up to
four legs (the `tick` construct's shapes). -/
section TickScanPaths
variable {a b c : TickShape} {τ : Type} {inst : DecidableEq α} {ret : Retries} {σ : Type}
  (ins : TickShape)
theorem co_tick_scan_0_sing_sr (x : TickedOf (CoupleSem L mem pacing Tc Td hjT).Ticked (CoupleSem L mem pacing Tc Td hjT).TickStream ℓ (ins)) (g : Fin (mem ℓ) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (ins) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) × BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (.sing τ)) (init : SeedOf sts) :
    ((CoupleSem L mem pacing Tc Td hjT).tick_scan (ℓ := ℓ) sts ins (.sing τ) x g init).sr = (SchedSem L mem pacing).tick_scan sts ins (.sing τ) (CoTickI.srLegs ins x) (CoTickI.gS sts ins (.sing τ) g) init := rfl
theorem co_tick_scan_0_sing_rr (x : TickedOf (CoupleSem L mem pacing Tc Td hjT).Ticked (CoupleSem L mem pacing Tc Td hjT).TickStream ℓ (ins)) (g : Fin (mem ℓ) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (ins) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) × BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (.sing τ)) (init : SeedOf sts) :
    ((CoupleSem L mem pacing Tc Td hjT).tick_scan (ℓ := ℓ) sts ins (.sing τ) x g init).rr = (Values L mem).tick_scan sts ins (.sing τ) (CoTickI.rrLegs ins x) (CoTickI.gR sts ins (.sing τ) g) init := rfl
theorem co_tick_scan_0_sing_wf (x : TickedOf (CoupleSem L mem pacing Tc Td hjT).Ticked (CoupleSem L mem pacing Tc Td hjT).TickStream ℓ (ins)) (g : Fin (mem ℓ) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (ins) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) × BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (.sing τ)) (init : SeedOf sts) :
    ((CoupleSem L mem pacing Tc Td hjT).tick_scan (ℓ := ℓ) sts ins (.sing τ) x g init).wf = CoTickI.scanWf sts ins (.sing τ) x g := rfl
theorem co_tick_scan_0_stream_sr (x : TickedOf (CoupleSem L mem pacing Tc Td hjT).Ticked (CoupleSem L mem pacing Tc Td hjT).TickStream ℓ (ins)) (g : Fin (mem ℓ) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (ins) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) × BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (.stream α inst ord ret)) (init : SeedOf sts) :
    ((CoupleSem L mem pacing Tc Td hjT).tick_scan (ℓ := ℓ) sts ins (.stream α inst ord ret) x g init).sr = (SchedSem L mem pacing).tick_scan sts ins (.stream α inst ord ret) (CoTickI.srLegs ins x) (CoTickI.gS sts ins (.stream α inst ord ret) g) init := rfl
theorem co_tick_scan_0_stream_rr (x : TickedOf (CoupleSem L mem pacing Tc Td hjT).Ticked (CoupleSem L mem pacing Tc Td hjT).TickStream ℓ (ins)) (g : Fin (mem ℓ) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (ins) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) × BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (.stream α inst ord ret)) (init : SeedOf sts) :
    ((CoupleSem L mem pacing Tc Td hjT).tick_scan (ℓ := ℓ) sts ins (.stream α inst ord ret) x g init).rr = (Values L mem).tick_scan sts ins (.stream α inst ord ret) (CoTickI.rrLegs ins x) (CoTickI.gR sts ins (.stream α inst ord ret) g) init := rfl
theorem co_tick_scan_0_stream_wf (x : TickedOf (CoupleSem L mem pacing Tc Td hjT).Ticked (CoupleSem L mem pacing Tc Td hjT).TickStream ℓ (ins)) (g : Fin (mem ℓ) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (ins) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) × BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (.stream α inst ord ret)) (init : SeedOf sts) :
    ((CoupleSem L mem pacing Tc Td hjT).tick_scan (ℓ := ℓ) sts ins (.stream α inst ord ret) x g init).wf = CoTickI.scanWf sts ins (.stream α inst ord ret) x g := rfl
theorem co_tick_scan_1_sing_sr (x : TickedOf (CoupleSem L mem pacing Tc Td hjT).Ticked (CoupleSem L mem pacing Tc Td hjT).TickStream ℓ (ins)) (g : Fin (mem ℓ) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (ins) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) × BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (.pair (.sing τ) a)) (init : SeedOf sts) :
    (((CoupleSem L mem pacing Tc Td hjT).tick_scan (ℓ := ℓ) sts ins (.pair (.sing τ) a) x g init).1).sr = ((SchedSem L mem pacing).tick_scan sts ins (.pair (.sing τ) a) (CoTickI.srLegs ins x) (CoTickI.gS sts ins (.pair (.sing τ) a) g) init).1 := rfl
theorem co_tick_scan_1_sing_rr (x : TickedOf (CoupleSem L mem pacing Tc Td hjT).Ticked (CoupleSem L mem pacing Tc Td hjT).TickStream ℓ (ins)) (g : Fin (mem ℓ) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (ins) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) × BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (.pair (.sing τ) a)) (init : SeedOf sts) :
    (((CoupleSem L mem pacing Tc Td hjT).tick_scan (ℓ := ℓ) sts ins (.pair (.sing τ) a) x g init).1).rr = ((Values L mem).tick_scan sts ins (.pair (.sing τ) a) (CoTickI.rrLegs ins x) (CoTickI.gR sts ins (.pair (.sing τ) a) g) init).1 := rfl
theorem co_tick_scan_1_sing_wf (x : TickedOf (CoupleSem L mem pacing Tc Td hjT).Ticked (CoupleSem L mem pacing Tc Td hjT).TickStream ℓ (ins)) (g : Fin (mem ℓ) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (ins) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) × BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (.pair (.sing τ) a)) (init : SeedOf sts) :
    (((CoupleSem L mem pacing Tc Td hjT).tick_scan (ℓ := ℓ) sts ins (.pair (.sing τ) a) x g init).1).wf = CoTickI.scanWf sts ins (.pair (.sing τ) a) x g := rfl
theorem co_tick_scan_1_stream_sr (x : TickedOf (CoupleSem L mem pacing Tc Td hjT).Ticked (CoupleSem L mem pacing Tc Td hjT).TickStream ℓ (ins)) (g : Fin (mem ℓ) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (ins) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) × BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (.pair (.stream α inst ord ret) a)) (init : SeedOf sts) :
    (((CoupleSem L mem pacing Tc Td hjT).tick_scan (ℓ := ℓ) sts ins (.pair (.stream α inst ord ret) a) x g init).1).sr = ((SchedSem L mem pacing).tick_scan sts ins (.pair (.stream α inst ord ret) a) (CoTickI.srLegs ins x) (CoTickI.gS sts ins (.pair (.stream α inst ord ret) a) g) init).1 := rfl
theorem co_tick_scan_1_stream_rr (x : TickedOf (CoupleSem L mem pacing Tc Td hjT).Ticked (CoupleSem L mem pacing Tc Td hjT).TickStream ℓ (ins)) (g : Fin (mem ℓ) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (ins) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) × BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (.pair (.stream α inst ord ret) a)) (init : SeedOf sts) :
    (((CoupleSem L mem pacing Tc Td hjT).tick_scan (ℓ := ℓ) sts ins (.pair (.stream α inst ord ret) a) x g init).1).rr = ((Values L mem).tick_scan sts ins (.pair (.stream α inst ord ret) a) (CoTickI.rrLegs ins x) (CoTickI.gR sts ins (.pair (.stream α inst ord ret) a) g) init).1 := rfl
theorem co_tick_scan_1_stream_wf (x : TickedOf (CoupleSem L mem pacing Tc Td hjT).Ticked (CoupleSem L mem pacing Tc Td hjT).TickStream ℓ (ins)) (g : Fin (mem ℓ) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (ins) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) × BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (.pair (.stream α inst ord ret) a)) (init : SeedOf sts) :
    (((CoupleSem L mem pacing Tc Td hjT).tick_scan (ℓ := ℓ) sts ins (.pair (.stream α inst ord ret) a) x g init).1).wf = CoTickI.scanWf sts ins (.pair (.stream α inst ord ret) a) x g := rfl
theorem co_tick_scan_2_sing_sr (x : TickedOf (CoupleSem L mem pacing Tc Td hjT).Ticked (CoupleSem L mem pacing Tc Td hjT).TickStream ℓ (ins)) (g : Fin (mem ℓ) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (ins) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) × BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (.pair a (.sing τ))) (init : SeedOf sts) :
    (((CoupleSem L mem pacing Tc Td hjT).tick_scan (ℓ := ℓ) sts ins (.pair a (.sing τ)) x g init).2).sr = ((SchedSem L mem pacing).tick_scan sts ins (.pair a (.sing τ)) (CoTickI.srLegs ins x) (CoTickI.gS sts ins (.pair a (.sing τ)) g) init).2 := rfl
theorem co_tick_scan_2_sing_rr (x : TickedOf (CoupleSem L mem pacing Tc Td hjT).Ticked (CoupleSem L mem pacing Tc Td hjT).TickStream ℓ (ins)) (g : Fin (mem ℓ) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (ins) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) × BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (.pair a (.sing τ))) (init : SeedOf sts) :
    (((CoupleSem L mem pacing Tc Td hjT).tick_scan (ℓ := ℓ) sts ins (.pair a (.sing τ)) x g init).2).rr = ((Values L mem).tick_scan sts ins (.pair a (.sing τ)) (CoTickI.rrLegs ins x) (CoTickI.gR sts ins (.pair a (.sing τ)) g) init).2 := rfl
theorem co_tick_scan_2_sing_wf (x : TickedOf (CoupleSem L mem pacing Tc Td hjT).Ticked (CoupleSem L mem pacing Tc Td hjT).TickStream ℓ (ins)) (g : Fin (mem ℓ) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (ins) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) × BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (.pair a (.sing τ))) (init : SeedOf sts) :
    (((CoupleSem L mem pacing Tc Td hjT).tick_scan (ℓ := ℓ) sts ins (.pair a (.sing τ)) x g init).2).wf = CoTickI.scanWf sts ins (.pair a (.sing τ)) x g := rfl
theorem co_tick_scan_2_stream_sr (x : TickedOf (CoupleSem L mem pacing Tc Td hjT).Ticked (CoupleSem L mem pacing Tc Td hjT).TickStream ℓ (ins)) (g : Fin (mem ℓ) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (ins) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) × BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (.pair a (.stream α inst ord ret))) (init : SeedOf sts) :
    (((CoupleSem L mem pacing Tc Td hjT).tick_scan (ℓ := ℓ) sts ins (.pair a (.stream α inst ord ret)) x g init).2).sr = ((SchedSem L mem pacing).tick_scan sts ins (.pair a (.stream α inst ord ret)) (CoTickI.srLegs ins x) (CoTickI.gS sts ins (.pair a (.stream α inst ord ret)) g) init).2 := rfl
theorem co_tick_scan_2_stream_rr (x : TickedOf (CoupleSem L mem pacing Tc Td hjT).Ticked (CoupleSem L mem pacing Tc Td hjT).TickStream ℓ (ins)) (g : Fin (mem ℓ) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (ins) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) × BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (.pair a (.stream α inst ord ret))) (init : SeedOf sts) :
    (((CoupleSem L mem pacing Tc Td hjT).tick_scan (ℓ := ℓ) sts ins (.pair a (.stream α inst ord ret)) x g init).2).rr = ((Values L mem).tick_scan sts ins (.pair a (.stream α inst ord ret)) (CoTickI.rrLegs ins x) (CoTickI.gR sts ins (.pair a (.stream α inst ord ret)) g) init).2 := rfl
theorem co_tick_scan_2_stream_wf (x : TickedOf (CoupleSem L mem pacing Tc Td hjT).Ticked (CoupleSem L mem pacing Tc Td hjT).TickStream ℓ (ins)) (g : Fin (mem ℓ) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (ins) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) × BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (.pair a (.stream α inst ord ret))) (init : SeedOf sts) :
    (((CoupleSem L mem pacing Tc Td hjT).tick_scan (ℓ := ℓ) sts ins (.pair a (.stream α inst ord ret)) x g init).2).wf = CoTickI.scanWf sts ins (.pair a (.stream α inst ord ret)) x g := rfl
theorem co_tick_scan_21_sing_sr (x : TickedOf (CoupleSem L mem pacing Tc Td hjT).Ticked (CoupleSem L mem pacing Tc Td hjT).TickStream ℓ (ins)) (g : Fin (mem ℓ) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (ins) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) × BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (.pair a (.pair (.sing τ) b))) (init : SeedOf sts) :
    ((((CoupleSem L mem pacing Tc Td hjT).tick_scan (ℓ := ℓ) sts ins (.pair a (.pair (.sing τ) b)) x g init).2).1).sr = (((SchedSem L mem pacing).tick_scan sts ins (.pair a (.pair (.sing τ) b)) (CoTickI.srLegs ins x) (CoTickI.gS sts ins (.pair a (.pair (.sing τ) b)) g) init).2).1 := rfl
theorem co_tick_scan_21_sing_rr (x : TickedOf (CoupleSem L mem pacing Tc Td hjT).Ticked (CoupleSem L mem pacing Tc Td hjT).TickStream ℓ (ins)) (g : Fin (mem ℓ) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (ins) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) × BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (.pair a (.pair (.sing τ) b))) (init : SeedOf sts) :
    ((((CoupleSem L mem pacing Tc Td hjT).tick_scan (ℓ := ℓ) sts ins (.pair a (.pair (.sing τ) b)) x g init).2).1).rr = (((Values L mem).tick_scan sts ins (.pair a (.pair (.sing τ) b)) (CoTickI.rrLegs ins x) (CoTickI.gR sts ins (.pair a (.pair (.sing τ) b)) g) init).2).1 := rfl
theorem co_tick_scan_21_sing_wf (x : TickedOf (CoupleSem L mem pacing Tc Td hjT).Ticked (CoupleSem L mem pacing Tc Td hjT).TickStream ℓ (ins)) (g : Fin (mem ℓ) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (ins) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) × BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (.pair a (.pair (.sing τ) b))) (init : SeedOf sts) :
    ((((CoupleSem L mem pacing Tc Td hjT).tick_scan (ℓ := ℓ) sts ins (.pair a (.pair (.sing τ) b)) x g init).2).1).wf = CoTickI.scanWf sts ins (.pair a (.pair (.sing τ) b)) x g := rfl
theorem co_tick_scan_21_stream_sr (x : TickedOf (CoupleSem L mem pacing Tc Td hjT).Ticked (CoupleSem L mem pacing Tc Td hjT).TickStream ℓ (ins)) (g : Fin (mem ℓ) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (ins) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) × BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (.pair a (.pair (.stream α inst ord ret) b))) (init : SeedOf sts) :
    ((((CoupleSem L mem pacing Tc Td hjT).tick_scan (ℓ := ℓ) sts ins (.pair a (.pair (.stream α inst ord ret) b)) x g init).2).1).sr = (((SchedSem L mem pacing).tick_scan sts ins (.pair a (.pair (.stream α inst ord ret) b)) (CoTickI.srLegs ins x) (CoTickI.gS sts ins (.pair a (.pair (.stream α inst ord ret) b)) g) init).2).1 := rfl
theorem co_tick_scan_21_stream_rr (x : TickedOf (CoupleSem L mem pacing Tc Td hjT).Ticked (CoupleSem L mem pacing Tc Td hjT).TickStream ℓ (ins)) (g : Fin (mem ℓ) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (ins) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) × BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (.pair a (.pair (.stream α inst ord ret) b))) (init : SeedOf sts) :
    ((((CoupleSem L mem pacing Tc Td hjT).tick_scan (ℓ := ℓ) sts ins (.pair a (.pair (.stream α inst ord ret) b)) x g init).2).1).rr = (((Values L mem).tick_scan sts ins (.pair a (.pair (.stream α inst ord ret) b)) (CoTickI.rrLegs ins x) (CoTickI.gR sts ins (.pair a (.pair (.stream α inst ord ret) b)) g) init).2).1 := rfl
theorem co_tick_scan_21_stream_wf (x : TickedOf (CoupleSem L mem pacing Tc Td hjT).Ticked (CoupleSem L mem pacing Tc Td hjT).TickStream ℓ (ins)) (g : Fin (mem ℓ) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (ins) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) × BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (.pair a (.pair (.stream α inst ord ret) b))) (init : SeedOf sts) :
    ((((CoupleSem L mem pacing Tc Td hjT).tick_scan (ℓ := ℓ) sts ins (.pair a (.pair (.stream α inst ord ret) b)) x g init).2).1).wf = CoTickI.scanWf sts ins (.pair a (.pair (.stream α inst ord ret) b)) x g := rfl
theorem co_tick_scan_22_sing_sr (x : TickedOf (CoupleSem L mem pacing Tc Td hjT).Ticked (CoupleSem L mem pacing Tc Td hjT).TickStream ℓ (ins)) (g : Fin (mem ℓ) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (ins) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) × BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (.pair a (.pair b (.sing τ)))) (init : SeedOf sts) :
    ((((CoupleSem L mem pacing Tc Td hjT).tick_scan (ℓ := ℓ) sts ins (.pair a (.pair b (.sing τ))) x g init).2).2).sr = (((SchedSem L mem pacing).tick_scan sts ins (.pair a (.pair b (.sing τ))) (CoTickI.srLegs ins x) (CoTickI.gS sts ins (.pair a (.pair b (.sing τ))) g) init).2).2 := rfl
theorem co_tick_scan_22_sing_rr (x : TickedOf (CoupleSem L mem pacing Tc Td hjT).Ticked (CoupleSem L mem pacing Tc Td hjT).TickStream ℓ (ins)) (g : Fin (mem ℓ) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (ins) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) × BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (.pair a (.pair b (.sing τ)))) (init : SeedOf sts) :
    ((((CoupleSem L mem pacing Tc Td hjT).tick_scan (ℓ := ℓ) sts ins (.pair a (.pair b (.sing τ))) x g init).2).2).rr = (((Values L mem).tick_scan sts ins (.pair a (.pair b (.sing τ))) (CoTickI.rrLegs ins x) (CoTickI.gR sts ins (.pair a (.pair b (.sing τ))) g) init).2).2 := rfl
theorem co_tick_scan_22_sing_wf (x : TickedOf (CoupleSem L mem pacing Tc Td hjT).Ticked (CoupleSem L mem pacing Tc Td hjT).TickStream ℓ (ins)) (g : Fin (mem ℓ) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (ins) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) × BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (.pair a (.pair b (.sing τ)))) (init : SeedOf sts) :
    ((((CoupleSem L mem pacing Tc Td hjT).tick_scan (ℓ := ℓ) sts ins (.pair a (.pair b (.sing τ))) x g init).2).2).wf = CoTickI.scanWf sts ins (.pair a (.pair b (.sing τ))) x g := rfl
theorem co_tick_scan_22_stream_sr (x : TickedOf (CoupleSem L mem pacing Tc Td hjT).Ticked (CoupleSem L mem pacing Tc Td hjT).TickStream ℓ (ins)) (g : Fin (mem ℓ) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (ins) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) × BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (.pair a (.pair b (.stream α inst ord ret)))) (init : SeedOf sts) :
    ((((CoupleSem L mem pacing Tc Td hjT).tick_scan (ℓ := ℓ) sts ins (.pair a (.pair b (.stream α inst ord ret))) x g init).2).2).sr = (((SchedSem L mem pacing).tick_scan sts ins (.pair a (.pair b (.stream α inst ord ret))) (CoTickI.srLegs ins x) (CoTickI.gS sts ins (.pair a (.pair b (.stream α inst ord ret))) g) init).2).2 := rfl
theorem co_tick_scan_22_stream_rr (x : TickedOf (CoupleSem L mem pacing Tc Td hjT).Ticked (CoupleSem L mem pacing Tc Td hjT).TickStream ℓ (ins)) (g : Fin (mem ℓ) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (ins) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) × BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (.pair a (.pair b (.stream α inst ord ret)))) (init : SeedOf sts) :
    ((((CoupleSem L mem pacing Tc Td hjT).tick_scan (ℓ := ℓ) sts ins (.pair a (.pair b (.stream α inst ord ret))) x g init).2).2).rr = (((Values L mem).tick_scan sts ins (.pair a (.pair b (.stream α inst ord ret))) (CoTickI.rrLegs ins x) (CoTickI.gR sts ins (.pair a (.pair b (.stream α inst ord ret))) g) init).2).2 := rfl
theorem co_tick_scan_22_stream_wf (x : TickedOf (CoupleSem L mem pacing Tc Td hjT).Ticked (CoupleSem L mem pacing Tc Td hjT).TickStream ℓ (ins)) (g : Fin (mem ℓ) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (ins) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) × BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (.pair a (.pair b (.stream α inst ord ret)))) (init : SeedOf sts) :
    ((((CoupleSem L mem pacing Tc Td hjT).tick_scan (ℓ := ℓ) sts ins (.pair a (.pair b (.stream α inst ord ret))) x g init).2).2).wf = CoTickI.scanWf sts ins (.pair a (.pair b (.stream α inst ord ret))) x g := rfl
theorem co_tick_scan_221_sing_sr (x : TickedOf (CoupleSem L mem pacing Tc Td hjT).Ticked (CoupleSem L mem pacing Tc Td hjT).TickStream ℓ (ins)) (g : Fin (mem ℓ) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (ins) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) × BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (.pair a (.pair b (.pair (.sing τ) c)))) (init : SeedOf sts) :
    (((((CoupleSem L mem pacing Tc Td hjT).tick_scan (ℓ := ℓ) sts ins (.pair a (.pair b (.pair (.sing τ) c))) x g init).2).2).1).sr = ((((SchedSem L mem pacing).tick_scan sts ins (.pair a (.pair b (.pair (.sing τ) c))) (CoTickI.srLegs ins x) (CoTickI.gS sts ins (.pair a (.pair b (.pair (.sing τ) c))) g) init).2).2).1 := rfl
theorem co_tick_scan_221_sing_rr (x : TickedOf (CoupleSem L mem pacing Tc Td hjT).Ticked (CoupleSem L mem pacing Tc Td hjT).TickStream ℓ (ins)) (g : Fin (mem ℓ) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (ins) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) × BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (.pair a (.pair b (.pair (.sing τ) c)))) (init : SeedOf sts) :
    (((((CoupleSem L mem pacing Tc Td hjT).tick_scan (ℓ := ℓ) sts ins (.pair a (.pair b (.pair (.sing τ) c))) x g init).2).2).1).rr = ((((Values L mem).tick_scan sts ins (.pair a (.pair b (.pair (.sing τ) c))) (CoTickI.rrLegs ins x) (CoTickI.gR sts ins (.pair a (.pair b (.pair (.sing τ) c))) g) init).2).2).1 := rfl
theorem co_tick_scan_221_sing_wf (x : TickedOf (CoupleSem L mem pacing Tc Td hjT).Ticked (CoupleSem L mem pacing Tc Td hjT).TickStream ℓ (ins)) (g : Fin (mem ℓ) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (ins) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) × BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (.pair a (.pair b (.pair (.sing τ) c)))) (init : SeedOf sts) :
    (((((CoupleSem L mem pacing Tc Td hjT).tick_scan (ℓ := ℓ) sts ins (.pair a (.pair b (.pair (.sing τ) c))) x g init).2).2).1).wf = CoTickI.scanWf sts ins (.pair a (.pair b (.pair (.sing τ) c))) x g := rfl
theorem co_tick_scan_221_stream_sr (x : TickedOf (CoupleSem L mem pacing Tc Td hjT).Ticked (CoupleSem L mem pacing Tc Td hjT).TickStream ℓ (ins)) (g : Fin (mem ℓ) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (ins) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) × BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (.pair a (.pair b (.pair (.stream α inst ord ret) c)))) (init : SeedOf sts) :
    (((((CoupleSem L mem pacing Tc Td hjT).tick_scan (ℓ := ℓ) sts ins (.pair a (.pair b (.pair (.stream α inst ord ret) c))) x g init).2).2).1).sr = ((((SchedSem L mem pacing).tick_scan sts ins (.pair a (.pair b (.pair (.stream α inst ord ret) c))) (CoTickI.srLegs ins x) (CoTickI.gS sts ins (.pair a (.pair b (.pair (.stream α inst ord ret) c))) g) init).2).2).1 := rfl
theorem co_tick_scan_221_stream_rr (x : TickedOf (CoupleSem L mem pacing Tc Td hjT).Ticked (CoupleSem L mem pacing Tc Td hjT).TickStream ℓ (ins)) (g : Fin (mem ℓ) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (ins) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) × BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (.pair a (.pair b (.pair (.stream α inst ord ret) c)))) (init : SeedOf sts) :
    (((((CoupleSem L mem pacing Tc Td hjT).tick_scan (ℓ := ℓ) sts ins (.pair a (.pair b (.pair (.stream α inst ord ret) c))) x g init).2).2).1).rr = ((((Values L mem).tick_scan sts ins (.pair a (.pair b (.pair (.stream α inst ord ret) c))) (CoTickI.rrLegs ins x) (CoTickI.gR sts ins (.pair a (.pair b (.pair (.stream α inst ord ret) c))) g) init).2).2).1 := rfl
theorem co_tick_scan_221_stream_wf (x : TickedOf (CoupleSem L mem pacing Tc Td hjT).Ticked (CoupleSem L mem pacing Tc Td hjT).TickStream ℓ (ins)) (g : Fin (mem ℓ) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (ins) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) × BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (.pair a (.pair b (.pair (.stream α inst ord ret) c)))) (init : SeedOf sts) :
    (((((CoupleSem L mem pacing Tc Td hjT).tick_scan (ℓ := ℓ) sts ins (.pair a (.pair b (.pair (.stream α inst ord ret) c))) x g init).2).2).1).wf = CoTickI.scanWf sts ins (.pair a (.pair b (.pair (.stream α inst ord ret) c))) x g := rfl
theorem co_tick_scan_222_sing_sr (x : TickedOf (CoupleSem L mem pacing Tc Td hjT).Ticked (CoupleSem L mem pacing Tc Td hjT).TickStream ℓ (ins)) (g : Fin (mem ℓ) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (ins) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) × BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (.pair a (.pair b (.pair c (.sing τ))))) (init : SeedOf sts) :
    (((((CoupleSem L mem pacing Tc Td hjT).tick_scan (ℓ := ℓ) sts ins (.pair a (.pair b (.pair c (.sing τ)))) x g init).2).2).2).sr = ((((SchedSem L mem pacing).tick_scan sts ins (.pair a (.pair b (.pair c (.sing τ)))) (CoTickI.srLegs ins x) (CoTickI.gS sts ins (.pair a (.pair b (.pair c (.sing τ)))) g) init).2).2).2 := rfl
theorem co_tick_scan_222_sing_rr (x : TickedOf (CoupleSem L mem pacing Tc Td hjT).Ticked (CoupleSem L mem pacing Tc Td hjT).TickStream ℓ (ins)) (g : Fin (mem ℓ) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (ins) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) × BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (.pair a (.pair b (.pair c (.sing τ))))) (init : SeedOf sts) :
    (((((CoupleSem L mem pacing Tc Td hjT).tick_scan (ℓ := ℓ) sts ins (.pair a (.pair b (.pair c (.sing τ)))) x g init).2).2).2).rr = ((((Values L mem).tick_scan sts ins (.pair a (.pair b (.pair c (.sing τ)))) (CoTickI.rrLegs ins x) (CoTickI.gR sts ins (.pair a (.pair b (.pair c (.sing τ)))) g) init).2).2).2 := rfl
theorem co_tick_scan_222_sing_wf (x : TickedOf (CoupleSem L mem pacing Tc Td hjT).Ticked (CoupleSem L mem pacing Tc Td hjT).TickStream ℓ (ins)) (g : Fin (mem ℓ) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (ins) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) × BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (.pair a (.pair b (.pair c (.sing τ))))) (init : SeedOf sts) :
    (((((CoupleSem L mem pacing Tc Td hjT).tick_scan (ℓ := ℓ) sts ins (.pair a (.pair b (.pair c (.sing τ)))) x g init).2).2).2).wf = CoTickI.scanWf sts ins (.pair a (.pair b (.pair c (.sing τ)))) x g := rfl
theorem co_tick_scan_222_stream_sr (x : TickedOf (CoupleSem L mem pacing Tc Td hjT).Ticked (CoupleSem L mem pacing Tc Td hjT).TickStream ℓ (ins)) (g : Fin (mem ℓ) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (ins) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) × BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (.pair a (.pair b (.pair c (.stream α inst ord ret))))) (init : SeedOf sts) :
    (((((CoupleSem L mem pacing Tc Td hjT).tick_scan (ℓ := ℓ) sts ins (.pair a (.pair b (.pair c (.stream α inst ord ret)))) x g init).2).2).2).sr = ((((SchedSem L mem pacing).tick_scan sts ins (.pair a (.pair b (.pair c (.stream α inst ord ret)))) (CoTickI.srLegs ins x) (CoTickI.gS sts ins (.pair a (.pair b (.pair c (.stream α inst ord ret)))) g) init).2).2).2 := rfl
theorem co_tick_scan_222_stream_rr (x : TickedOf (CoupleSem L mem pacing Tc Td hjT).Ticked (CoupleSem L mem pacing Tc Td hjT).TickStream ℓ (ins)) (g : Fin (mem ℓ) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (ins) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) × BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (.pair a (.pair b (.pair c (.stream α inst ord ret))))) (init : SeedOf sts) :
    (((((CoupleSem L mem pacing Tc Td hjT).tick_scan (ℓ := ℓ) sts ins (.pair a (.pair b (.pair c (.stream α inst ord ret)))) x g init).2).2).2).rr = ((((Values L mem).tick_scan sts ins (.pair a (.pair b (.pair c (.stream α inst ord ret)))) (CoTickI.rrLegs ins x) (CoTickI.gR sts ins (.pair a (.pair b (.pair c (.stream α inst ord ret)))) g) init).2).2).2 := rfl
theorem co_tick_scan_222_stream_wf (x : TickedOf (CoupleSem L mem pacing Tc Td hjT).Ticked (CoupleSem L mem pacing Tc Td hjT).TickStream ℓ (ins)) (g : Fin (mem ℓ) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (ins) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (sts) × BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (.pair a (.pair b (.pair c (.stream α inst ord ret))))) (init : SeedOf sts) :
    (((((CoupleSem L mem pacing Tc Td hjT).tick_scan (ℓ := ℓ) sts ins (.pair a (.pair b (.pair c (.stream α inst ord ret)))) x g init).2).2).2).wf = CoTickI.scanWf sts ins (.pair a (.pair b (.pair c (.stream α inst ord ret)))) x g := rfl
end TickScanPaths

end InTick

theorem co_defer_tick_rr {σ : Type} (init : σ)
    (t : (CoupleSem L mem pacing Tc Td hjT).Ticked ℓ σ)
    :
    ((CoupleSem L mem pacing Tc Td hjT).defer_tick (ℓ := ℓ) init t).rr
      = (Values L mem).defer_tick (ℓ := ℓ) init t.rr := rfl

theorem co_defer_tick_sr {σ : Type} (init : σ)
    (t : (CoupleSem L mem pacing Tc Td hjT).Ticked ℓ σ) :
    ((CoupleSem L mem pacing Tc Td hjT).defer_tick (ℓ := ℓ) init t).sr
      = (SchedSem L mem pacing).defer_tick (ℓ := ℓ) init t.sr := rfl

theorem co_defer_tick_wf {σ : Type} (init : σ)
    (t : (CoupleSem L mem pacing Tc Td hjT).Ticked ℓ σ) :
    ((CoupleSem L mem pacing Tc Td hjT).defer_tick (ℓ := ℓ) init t).wf
      = t.wf := rfl

theorem co_flattenOrdered_rr {β : Type} [DecidableEq β]
    (t : (CoupleSem L mem pacing Tc Td hjT).Ticked ℓ (List β))
    :
    ((CoupleSem L mem pacing Tc Td hjT).flattenOrdered (ℓ := ℓ) t).rr
      = (Values L mem).flattenOrdered (ℓ := ℓ) t.rr := rfl

theorem co_flattenOrdered_sr {β : Type} [DecidableEq β]
    (t : (CoupleSem L mem pacing Tc Td hjT).Ticked ℓ (List β)) :
    ((CoupleSem L mem pacing Tc Td hjT).flattenOrdered (ℓ := ℓ) t).sr
      = (SchedSem L mem pacing).flattenOrdered (ℓ := ℓ) t.sr := rfl

theorem co_flattenOrdered_wf {β : Type} [DecidableEq β]
    (t : (CoupleSem L mem pacing Tc Td hjT).Ticked ℓ (List β)) :
    ((CoupleSem L mem pacing Tc Td hjT).flattenOrdered (ℓ := ℓ) t).wf
      = t.wf := rfl

theorem co_flattenUnordered_rr {β : Type} [DecidableEq β]
    (t : (CoupleSem L mem pacing Tc Td hjT).Ticked ℓ (List β))
    :
    ((CoupleSem L mem pacing Tc Td hjT).flattenUnordered (ℓ := ℓ) t).rr
      = (Values L mem).flattenUnordered (ℓ := ℓ) t.rr := rfl

theorem co_flattenUnordered_sr {β : Type} [DecidableEq β]
    (t : (CoupleSem L mem pacing Tc Td hjT).Ticked ℓ (List β)) :
    ((CoupleSem L mem pacing Tc Td hjT).flattenUnordered (ℓ := ℓ) t).sr
      = (SchedSem L mem pacing).flattenUnordered (ℓ := ℓ) t.sr
      := rfl

theorem co_flattenUnordered_wf {β : Type} [DecidableEq β]
    (t : (CoupleSem L mem pacing Tc Td hjT).Ticked ℓ (List β)) :
    ((CoupleSem L mem pacing Tc Td hjT).flattenUnordered (ℓ := ℓ) t).wf
      = t.wf := rfl

theorem co_allTicks_rr {β : Type} [DecidableEq β]
    {ord : StrOrd}
    (bs : (CoupleSem L mem pacing Tc Td hjT).TickStream ℓ β ord .exactlyOnce)
    :
    ((CoupleSem L mem pacing Tc Td hjT).allTicks (ℓ := ℓ) (ord := ord) bs).rr
      = (Values L mem).allTicks (ℓ := ℓ) (ord := ord) bs.rr := rfl

theorem co_allTicks_sr {β : Type} [DecidableEq β]
    {ord : StrOrd}
    (bs : (CoupleSem L mem pacing Tc Td hjT).TickStream ℓ β ord .exactlyOnce) :
    ((CoupleSem L mem pacing Tc Td hjT).allTicks (ℓ := ℓ) (ord := ord) bs).sr
      = (SchedSem L mem pacing).allTicks (ℓ := ℓ) (ord := ord) bs.sr
      := rfl

theorem co_allTicks_wf {β : Type} [DecidableEq β]
    {ord : StrOrd}
    (bs : (CoupleSem L mem pacing Tc Td hjT).TickStream ℓ β ord .exactlyOnce) :
    ((CoupleSem L mem pacing Tc Td hjT).allTicks (ℓ := ℓ) (ord := ord) bs).wf
      = bs.wf := rfl

theorem co_sample_every_rr {α : Type} [DecidableEq α]
    (t : (CoupleSem L mem pacing Tc Td hjT).Ticked ℓ (Option α))
    (times : (CoupleSem L mem pacing Tc Td hjT).SampleDec (mem ℓ)) :
    ((CoupleSem L mem pacing Tc Td hjT).sample_every t times).rr
      = (Values L mem).sample_every (ℓ := ℓ) t.rr times := rfl

theorem co_sample_every_sr {α : Type} [DecidableEq α]
    (t : (CoupleSem L mem pacing Tc Td hjT).Ticked ℓ (Option α))
    (times : (CoupleSem L mem pacing Tc Td hjT).SampleDec (mem ℓ)) :
    ((CoupleSem L mem pacing Tc Td hjT).sample_every t times).sr
      = (SchedSem L mem pacing).sample_every (ℓ := ℓ) t.sr times := rfl

theorem co_sample_every_wf {α : Type} [DecidableEq α]
    (t : (CoupleSem L mem pacing Tc Td hjT).Ticked ℓ (Option α))
    (times : (CoupleSem L mem pacing Tc Td hjT).SampleDec (mem ℓ)) :
    ((CoupleSem L mem pacing Tc Td hjT).sample_every t times).wf = t.wf := rfl

theorem co_timeout_snapshot_rr {α : Type} [DecidableEq α]
    {ord : StrOrd} {ret : Retries}
    (s : (CoupleSem L mem pacing Tc Td hjT).Stream ℓ α ord ret)
    (verd : (CoupleSem L mem pacing Tc Td hjT).TimerDec (mem ℓ)) :
    ((CoupleSem L mem pacing Tc Td hjT).timeout_snapshot s verd).rr
      = (Values L mem).timeout_snapshot (ℓ := ℓ) (ord := ord)
          (ret := ret) s.rr verd := rfl

theorem co_timeout_snapshot_sr {α : Type} [DecidableEq α]
    {ord : StrOrd} {ret : Retries}
    (s : (CoupleSem L mem pacing Tc Td hjT).Stream ℓ α ord ret)
    (verd : (CoupleSem L mem pacing Tc Td hjT).TimerDec (mem ℓ)) :
    ((CoupleSem L mem pacing Tc Td hjT).timeout_snapshot s verd).sr
      = (SchedSem L mem pacing).timeout_snapshot (ℓ := ℓ) (ord := ord)
          (ret := ret) s.sr verd := rfl

theorem co_timeout_snapshot_wf {α : Type} [DecidableEq α]
    {ord : StrOrd} {ret : Retries}
    (s : (CoupleSem L mem pacing Tc Td hjT).Stream ℓ α ord ret)
    (verd : (CoupleSem L mem pacing Tc Td hjT).TimerDec (mem ℓ)) :
    ((CoupleSem L mem pacing Tc Td hjT).timeout_snapshot s verd).wf = True
    := rfl

theorem co_source_interval_batch_rr
    (pulses : (CoupleSem L mem pacing Tc Td hjT).PulseDec (mem ℓ)) :
    ((CoupleSem L mem pacing Tc Td hjT).source_interval_batch
        (ℓ := ℓ) pulses).rr
      = (Values L mem).source_interval_batch (ℓ := ℓ) pulses := rfl

theorem co_source_interval_batch_sr
    (pulses : (CoupleSem L mem pacing Tc Td hjT).PulseDec (mem ℓ)) :
    ((CoupleSem L mem pacing Tc Td hjT).source_interval_batch
        (ℓ := ℓ) pulses).sr
      = (SchedSem L mem pacing).source_interval_batch (ℓ := ℓ) pulses
    := rfl

theorem co_source_interval_batch_wf
    (pulses : (CoupleSem L mem pacing Tc Td hjT).PulseDec (mem ℓ)) :
    ((CoupleSem L mem pacing Tc Td hjT).source_interval_batch
        (ℓ := ℓ) pulses).wf = True := rfl

/-! ## Knot projections (`HydroSem.fix` wrapper-keyed) -/

section FixProj

variable {Γ : HydroSem L mem → Type}

/-- The knot's machine diagonal, *named* — the naming identities keep
it opaque (it only appears inside derived-decision carrier arguments,
so the projection simp set must not re-normalize the whole machine
body once per knot). -/
def CoStream.fixSrC {α : Type} [DecidableEq α] {ord : StrOrd}
    {ret : Retries} (caps : Γ (CoupleSem L mem pacing Tc Td hjT))
    (body : ∀ H', Γ H' → H'.Stream ℓ α ord ret → H'.Stream ℓ α ord ret) :
    (SchedSem L mem pacing).Stream ℓ α ord ret :=
  (SchedSem L mem pacing).fix_stream (ℓ := ℓ) ()
    (fun x => (body _ caps (CoStream.schedC x)).sr)

def CoTickSing.fixSrC {σ : Type}
    (caps : Γ (CoupleSem L mem pacing Tc Td hjT))
    (body : ∀ H', Γ H' → H'.Ticked ℓ σ →
      H'.Ticked ℓ σ) :
    (SchedSem L mem pacing).Ticked ℓ σ :=
  (SchedSem L mem pacing).fix_tick (ℓ := ℓ) ()
    (fun x => (body _ caps (CoTickSing.schedC x)).sr)

theorem co_hfix_stream_rr {α : Type} [DecidableEq α]
    {ord : StrOrd} {ret : Retries}
    (df : (CoupleSem L mem pacing Tc Td hjT).FixDec)
    (caps : Γ (CoupleSem L mem pacing Tc Td hjT))
    (body : ∀ H', Γ H' → H'.Stream ℓ α ord ret → H'.Stream ℓ α ord ret) :
    ((CoupleSem L mem pacing Tc Td hjT).fix (ℓ := ℓ) df caps body).rr
      = (Values L mem).fix_stream (ℓ := ℓ) (α := α) (ord := ord)
          (ret := ret) (Td + 1)
          (fun v => (body _ caps (CoStream.probeC
            (CoStream.fixSrC caps body) v)).rr) := rfl

theorem co_hfix_stream_sr {α : Type} [DecidableEq α]
    {ord : StrOrd} {ret : Retries}
    (df : (CoupleSem L mem pacing Tc Td hjT).FixDec)
    (caps : Γ (CoupleSem L mem pacing Tc Td hjT))
    (body : ∀ H', Γ H' → H'.Stream ℓ α ord ret → H'.Stream ℓ α ord ret) :
    ((CoupleSem L mem pacing Tc Td hjT).fix (ℓ := ℓ) df caps body).sr
      = CoStream.fixSrC caps body := rfl

/-- The named diagonal, unfolded (only the machine-naming identities
open it; the decision-naming identities keep it folded so the pins of
in-knot and top-level site occurrences coincide). -/
theorem co_fixSrC_eq {α : Type} [DecidableEq α]
    {ord : StrOrd} {ret : Retries}
    (caps : Γ (CoupleSem L mem pacing Tc Td hjT))
    (body : ∀ H', Γ H' → H'.Stream ℓ α ord ret → H'.Stream ℓ α ord ret) :
    CoStream.fixSrC caps body
      = (SchedSem L mem pacing).fix_stream (ℓ := ℓ) (α := α)
          (ord := ord) (ret := ret) ()
          (fun x => (body _ caps (CoStream.schedC x)).sr) := rfl

theorem co_hfix_tick_rr {σ : Type}
    (df : (CoupleSem L mem pacing Tc Td hjT).FixDec)
    (caps : Γ (CoupleSem L mem pacing Tc Td hjT))
    (body : ∀ H', Γ H' → H'.Ticked ℓ σ →
      H'.Ticked ℓ σ) :
    ((CoupleSem L mem pacing Tc Td hjT).fixTick (ℓ := ℓ) df caps
        body).rr
      = (Values L mem).fix_tick (ℓ := ℓ) (σ := σ) (Td + 1)
          (fun v => (body _ caps (CoTickSing.probeC
            (CoTickSing.fixSrC caps body) v)).rr) := rfl

theorem co_hfix_tick_sr {σ : Type}
    (df : (CoupleSem L mem pacing Tc Td hjT).FixDec)
    (caps : Γ (CoupleSem L mem pacing Tc Td hjT))
    (body : ∀ H', Γ H' → H'.Ticked ℓ σ →
      H'.Ticked ℓ σ) :
    ((CoupleSem L mem pacing Tc Td hjT).fixTick (ℓ := ℓ) df caps
        body).sr
      = CoTickSing.fixSrC caps body := rfl

theorem co_tick_fixSrC_eq {σ : Type}
    (caps : Γ (CoupleSem L mem pacing Tc Td hjT))
    (body : ∀ H', Γ H' → H'.Ticked ℓ σ →
      H'.Ticked ℓ σ) :
    CoTickSing.fixSrC caps body
      = (SchedSem L mem pacing).fix_tick (ℓ := ℓ) (σ := σ) ()
          (fun x => (body _ caps (CoTickSing.schedC x)).sr) := rfl

end FixProj

end Ops

/-! ## `HydroSem.fix` wrappers at the target instances (clean binders:
no section variable may be captured, or `simp` never instantiates
them) -/

section TargetFix

variable {Γ : HydroSem L mem → Type}

theorem values_hfix_stream' {α : Type} [DecidableEq α]
    {ord : StrOrd} {ret : Retries}
    (df : (Values L mem).FixDec) (caps : Γ (Values L mem))
    (body : ∀ H', Γ H' → H'.Stream ℓ α ord ret → H'.Stream ℓ α ord ret) :
    (Values L mem).fix (ℓ := ℓ) df caps body
      = (Values L mem).fix_stream (ℓ := ℓ) df
          (fun s => body _ caps s) := rfl

theorem values_hfix_tick' {σ : Type}
    (df : (Values L mem).FixDec) (caps : Γ (Values L mem))
    (body : ∀ H', Γ H' → H'.Ticked ℓ σ →
      H'.Ticked ℓ σ) :
    (Values L mem).fixTick (ℓ := ℓ) df caps body
      = (Values L mem).fix_tick (ℓ := ℓ) df
          (fun s => body _ caps s) := rfl

/-! ## Unit-decision normalizers (instance-generic — `Values`/`SchedSem`
ops whose decision type is `Unit` eta-reduce) -/

theorem values_broadcast_closed_unit {α : Type} [DecidableEq α]
    {ord : StrOrd} {ret : Retries}
    (u : (Values L mem).TransportDec (mem p) (mem c))
    (s : (Values L mem).Stream c α ord ret) :
    (Values L mem).broadcast_closed (c := c) (p := p) (ord := ord) (ret := ret)
        u s
      = (Values L mem).broadcast_closed (c := c) (p := p) (ord := ord)
          (ret := ret) () s := rfl

theorem values_demux_unit {α : Type} [DecidableEq α]
    {ord : StrOrd} (u : (Values L mem).TransportDec (mem p) (mem c))
    (s : (Values L mem).Stream c (Nat × α) ord .exactlyOnce)
    (addr : Fin (mem p) → Nat) :
    (Values L mem).demux (c := c) (p := p) (ord := ord) u s addr
      = (Values L mem).demux (c := c) (p := p) (ord := ord) () s addr
      := rfl

theorem sched_assume_ordering_unit {α : Type} [DecidableEq α]
    (u : (SchedSem L mem pacing).OrderSelDec (mem ℓ) α)
    (s : (SchedSem L mem pacing).Stream ℓ α .noOrder .exactlyOnce) :
    (SchedSem L mem pacing).assume_ordering (ℓ := ℓ) s u
      = (SchedSem L mem pacing).assume_ordering (ℓ := ℓ) s () := rfl

theorem sched_snapshot_unit {α σ : Type} [DecidableEq α]
    {ord : StrOrd} {ret : Retries} {b : SingBound σ}
    (u : (SchedSem L mem pacing).SnapDec (mem ℓ) α ord)
    (s : (SchedSem L mem pacing).Singleton ℓ α σ ord ret b) :
    (SchedSem L mem pacing).snapshot (ℓ := ℓ) (ord := ord) (ret := ret)
        (b := b) s u
      = (SchedSem L mem pacing).snapshot (ℓ := ℓ) (ord := ord)
          (ret := ret) (b := b) s () := rfl

theorem sched_batch_unit {α : Type} [DecidableEq α]
    (u : (SchedSem L mem pacing).BatchDec (mem ℓ) α)
    (s : (SchedSem L mem pacing).Stream ℓ α .noOrder .exactlyOnce) :
    (SchedSem L mem pacing).batch (ℓ := ℓ) s u
      = (SchedSem L mem pacing).batch (ℓ := ℓ) s () := rfl

theorem sched_batch_ordered_unit {α : Type} [DecidableEq α]
    (u : (SchedSem L mem pacing).OrdBatchDec (mem ℓ))
    (s : (SchedSem L mem pacing).Stream ℓ α .totalOrder .exactlyOnce) :
    (SchedSem L mem pacing).batch_ordered (ℓ := ℓ) s u
      = (SchedSem L mem pacing).batch_ordered (ℓ := ℓ) s () := rfl

theorem sched_fix_stream_unit {α : Type} [DecidableEq α]
    {ord : StrOrd} {ret : Retries}
    (u : (SchedSem L mem pacing).FixDec)
    (body : (SchedSem L mem pacing).Stream ℓ α ord ret →
      (SchedSem L mem pacing).Stream ℓ α ord ret) :
    (SchedSem L mem pacing).fix_stream (ℓ := ℓ) u body
      = (SchedSem L mem pacing).fix_stream (ℓ := ℓ) () body := rfl

theorem sched_fix_tick_unit {σ : Type}
    (u : (SchedSem L mem pacing).FixDec)
    (body : (SchedSem L mem pacing).Ticked ℓ σ →
      (SchedSem L mem pacing).Ticked ℓ σ) :
    (SchedSem L mem pacing).fix_tick (ℓ := ℓ) u body
      = (SchedSem L mem pacing).fix_tick (ℓ := ℓ) () body := rfl

theorem sched_hfix_stream' {α : Type} [DecidableEq α]
    {ord : StrOrd} {ret : Retries}
    (df : (SchedSem L mem pacing).FixDec)
    (caps : Γ (SchedSem L mem pacing))
    (body : ∀ H', Γ H' → H'.Stream ℓ α ord ret → H'.Stream ℓ α ord ret) :
    (SchedSem L mem pacing).fix (ℓ := ℓ) df caps body
      = (SchedSem L mem pacing).fix_stream (ℓ := ℓ) df
          (fun s => body _ caps s) := rfl

theorem sched_hfix_tick' {σ : Type}
    (df : (SchedSem L mem pacing).FixDec)
    (caps : Γ (SchedSem L mem pacing))
    (body : ∀ H', Γ H' → H'.Ticked ℓ σ →
      H'.Ticked ℓ σ) :
    (SchedSem L mem pacing).fixTick (ℓ := ℓ) df caps body
      = (SchedSem L mem pacing).fix_tick (ℓ := ℓ) df
          (fun s => body _ caps s) := rfl

end TargetFix

/-! ## The transfer macros -/

open Lean

/-- The projection simp set's rules: `rr`/`sr` through every op (one
array, so the macro's quotation stays small — and `HydroGenKnot`'s
naming engine can share the list). -/
def coSimpRules : Array Name := #[
  ``co_input_rr, ``co_input_sr, ``co_tick_input_rr,
  ``co_tick_input_sr, ``co_probeC_rr, ``co_probeC_sr,
  ``co_tick_probeC_rr, ``co_tick_probeC_sr, ``co_mkC_rr,
  ``co_mkC_sr, ``co_tick_mkC_rr, ``co_tick_mkC_sr,
  ``co_mkC_rr_raw, ``co_mkC_sr_raw, ``co_tick_mkC_rr_raw,
  ``co_tick_mkC_sr_raw, ``co_embedS_sr_raw, ``co_tick_embedS_sr_raw,
  ``co_probe2_rr_raw, ``co_probe2_sr_raw, ``co_tick_probe2_rr_raw,
  ``co_tick_probe2_sr_raw, ``co_schedC_sr, ``co_tick_schedC_sr,
  ``co_lowerC_rr, ``co_lowerC_sr, ``co_tick_lowerC_rr,
  ``co_tick_lowerC_sr, ``co_map_rr, ``co_map_sr,
  ``co_filterMap_rr, ``co_filterMap_sr, ``co_broadcast_closed_rr,
  ``co_broadcast_closed_sr, ``co_demux_rr, ``co_demux_sr,
  ``co_values_rr, ``co_values_sr, ``co_weaken_retries_rr,
  ``co_weaken_retries_sr, ``co_union_rr, ``co_union_sr,
  ``co_assume_ordering_rr, ``co_assume_ordering_sr, ``co_fold_rr,
  ``co_fold_sr, ``co_fold_monotone_rr, ``co_fold_monotone_sr,
  ``co_snapshot_rr, ``co_snapshot_sr, ``co_batch_rr,
  ``co_batch_sr, ``co_batch_ordered_rr, ``co_batch_ordered_sr,
  ``co_mapTick_rr,
  ``co_mapTick_sr, ``co_zipTick_rr, ``co_zipTick_sr,
  ``co_defer_tick_rr,
  ``co_defer_tick_sr, ``co_bmap_rr, ``co_bmap_sr,
  ``co_bfilterMap_rr, ``co_bfilterMap_sr, ``co_bflatMapOrdered_rr,
  ``co_bflatMapOrdered_sr, ``co_bflatMapUnordered_rr, ``co_bflatMapUnordered_sr,
  ``co_bofList_rr, ``co_bofList_sr, ``co_benumerate_rr,
  ``co_benumerate_sr, ``co_bcrossSingleton_rr, ``co_bcrossSingleton_sr,
  ``co_bchain_rr, ``co_bchain_sr, ``co_bweakenOrder_rr,
  ``co_bweakenOrder_sr, ``co_bfilter_rr, ``co_bfilter_sr,
  ``co_bkeyedFold_rr, ``co_bkeyedFold_sr, ``co_bkeys_rr,
  ``co_bkeys_sr, ``co_bjoin_rr, ``co_bjoin_sr,
  ``co_bantiJoin_rr, ``co_bantiJoin_sr, ``co_bfilterNotIn_rr, ``co_bmax_rr, ``co_bmax_sr,
  ``co_bfilterNotIn_sr, ``co_bfilterIf_rr, ``co_bfilterIf_sr,
  ``co_ite_bstream_sr, ``co_ite_bstream_rr, ``co_ite_bsing_sr,
  ``co_ite_bsing_rr, ``fst_ite, ``snd_ite,
  ``co_tick_scan_0_sing_sr,
  ``co_tick_scan_0_sing_rr, ``co_tick_scan_0_stream_sr, ``co_tick_scan_0_stream_rr,
  ``co_tick_scan_1_sing_sr, ``co_tick_scan_1_sing_rr, ``co_tick_scan_1_stream_sr,
  ``co_tick_scan_1_stream_rr, ``co_tick_scan_2_sing_sr, ``co_tick_scan_2_sing_rr,
  ``co_tick_scan_2_stream_sr, ``co_tick_scan_2_stream_rr, ``co_tick_scan_21_sing_sr,
  ``co_tick_scan_21_sing_rr, ``co_tick_scan_21_stream_sr, ``co_tick_scan_21_stream_rr,
  ``co_tick_scan_22_sing_sr, ``co_tick_scan_22_sing_rr, ``co_tick_scan_22_stream_sr,
  ``co_tick_scan_22_stream_rr, ``co_tick_scan_221_sing_sr, ``co_tick_scan_221_sing_rr,
  ``co_tick_scan_221_stream_sr, ``co_tick_scan_221_stream_rr, ``co_tick_scan_222_sing_sr,
  ``co_tick_scan_222_sing_rr, ``co_tick_scan_222_stream_sr, ``co_tick_scan_222_stream_rr,
  ``CoTickI.srLegs_pair, ``CoTickI.srLegs_sing, ``CoTickI.srLegs_stream,
  ``CoTickI.rrLegs_pair, ``CoTickI.rrLegs_sing, ``CoTickI.rrLegs_stream,
  ``CoTickI.gS_def, ``CoTickI.gR_def, ``CoTickI.gC_def,
  ``CoTickI.projSR_pair, ``CoTickI.projSR_sing, ``CoTickI.projSR_stream,
  ``CoTickI.projRR_pair, ``CoTickI.projRR_sing, ``CoTickI.projRR_stream,
  ``CoTickI.embedSR_pair, ``CoTickI.embedSR_sing_sr, ``CoTickI.embedSR_sing_rr,
  ``CoTickI.embedSR_stream_sr, ``CoTickI.embedRR_pair, ``CoTickI.embedRR_sing_sr,
  ``CoTickI.embedRR_sing_rr, ``CoTickI.embedRR_stream_rr, ``CoTickI.embed2_pair,
  ``CoTickI.embed2_sing_sr, ``CoTickI.embed2_sing_rr, ``CoTickI.embed2_stream_sr,
  ``CoTickI.embed2_stream_rr, ``CoTickI.coerce_pair, ``CoTickI.coerce_sing,
  ``co_bcount_rr, ``co_bcount_sr, ``co_bfold_rr,
  ``co_bfold_sr, ``co_bfirst_rr, ``co_bfirst_sr,
  ``co_bsPure_rr, ``co_bsPure_sr, ``co_bsMap_rr,
  ``co_bsMap_sr, ``co_bsZip_rr, ``co_bsZip_sr,
  ``co_boMap_rr, ``co_boMap_sr, ``co_boUnwrapOr_rr,
  ``co_boUnwrapOr_sr, ``co_boFilter_rr, ``co_boFilter_sr,
  ``co_boIsSome_rr, ``co_boIsSome_sr, ``co_sample_every_rr,
  ``co_sample_every_sr, ``co_timeout_snapshot_rr, ``co_timeout_snapshot_sr,
  ``co_source_interval_batch_rr, ``co_source_interval_batch_sr,
  ``co_flattenOrdered_rr, ``co_flattenOrdered_sr,
  ``co_flattenUnordered_rr,
  ``co_flattenUnordered_sr, ``co_allTicks_rr, ``co_allTicks_sr,
  ``co_hfix_stream_rr, ``co_hfix_stream_sr, ``co_hfix_tick_rr,
  ``co_hfix_tick_sr, ``values_hfix_stream', ``values_hfix_tick',
  ``sched_hfix_stream', ``sched_hfix_tick', ``values_broadcast_closed_unit,
  ``values_demux_unit, ``sched_assume_ordering_unit,
  ``sched_snapshot_unit, ``sched_batch_unit, ``sched_batch_ordered_unit,
  ``sched_fix_stream_unit, ``sched_fix_tick_unit,
  ``BoundedOf, ``TickedOf]

/-- The projection simp set: pushes `rr`/`sr` through every op. -/
macro "co_simp" "[" ids:Lean.Parser.Tactic.simpLemma,* "]" : tactic => do
  let rs ← coSimpRules.mapM fun n => `(Lean.Parser.Tactic.simpLemma| $(mkIdent n):ident)
  `(tactic| simp only [$ids,*, $rs,*])

/-- The transfer identity closer: projections + reducible `rfl` (the
default-transparency fallback types the decision-record pins — witness
metavariables' raw-carrier assignments — after both sides are already
in simp-normal form). -/
macro "co_transfer" "[" ids:Lean.Parser.Tactic.simpLemma,* "]" :
    tactic =>
  `(tactic| (co_simp [$ids,*]; first
    | done
    | with_reducible rfl
    | exact rfl))

/-- The `wf` normalizer's rules. -/
def coWfSimpRules : Array Name := #[
  ``co_input_wf, ``co_tick_input_wf, ``co_mkC_wf,
  ``co_tick_mkC_wf, ``co_mkC_wf_raw, ``co_tick_mkC_wf_raw,
  ``co_map_wf, ``co_filterMap_wf, ``co_broadcast_closed_wf,
  ``co_demux_wf, ``co_values_wf, ``co_weaken_retries_wf,
  ``co_union_wf, ``co_assume_ordering_wf, ``co_fold_wf,
  ``co_fold_monotone_wf, ``co_snapshot_wf, ``co_batch_wf,
  ``co_batch_ordered_wf,
  ``co_mapTick_wf,
  ``co_zipTick_wf, ``co_defer_tick_wf,
  ``co_sample_every_wf, ``co_bmap_wf, ``co_bfilterMap_wf,
  ``co_bflatMapOrdered_wf, ``co_bflatMapUnordered_wf, ``co_bofList_wf,
  ``co_benumerate_wf, ``co_bcrossSingleton_wf, ``co_bchain_wf,
  ``co_bweakenOrder_wf, ``co_bfilter_wf, ``co_bkeyedFold_wf,
  ``co_bkeys_wf, ``co_bjoin_wf, ``co_bantiJoin_wf,
  ``co_bfilterNotIn_wf, ``co_bmax_wf, ``co_ite_bstream_sr, ``co_ite_bstream_rr,
  ``co_ite_bstream_wf, ``co_ite_bsing_sr, ``co_ite_bsing_rr,
  ``co_ite_bsing_wf, ``fst_ite, ``snd_ite,
  ``co_bfilterIf_wf, ``co_tick_scan_0_sing_wf,
  ``co_tick_scan_0_stream_wf, ``co_tick_scan_1_sing_wf, ``co_tick_scan_1_stream_wf,
  ``co_tick_scan_2_sing_wf, ``co_tick_scan_2_stream_wf, ``co_tick_scan_21_sing_wf,
  ``co_tick_scan_21_stream_wf, ``co_tick_scan_22_sing_wf, ``co_tick_scan_22_stream_wf,
  ``co_tick_scan_221_sing_wf, ``co_tick_scan_221_stream_wf, ``co_tick_scan_222_sing_wf,
  ``co_tick_scan_222_stream_wf, ``CoTickI.scanWf_def, ``CoTickI.Wf_pair,
  ``CoTickI.Wf_sing, ``CoTickI.Wf_stream, ``CoTick.allEO_sing,
  ``CoTick.allEO_stream_eo, ``CoTick.allEO_pair_eq, ``Bool.and_self,
  ``Bool.true_and, ``Bool.and_true, ``CoTickI.W1_def,
  ``CoTickI.W2_def, ``CoTickI.W3_def, ``CoTickI.gS_def,
  ``CoTickI.gR_def, ``CoTickI.gC_def, ``CoTickI.projSR_pair,
  ``CoTickI.projSR_sing, ``CoTickI.projSR_stream, ``CoTickI.projRR_pair,
  ``CoTickI.projRR_sing, ``CoTickI.projRR_stream, ``CoTickI.WfB_pair,
  ``CoTickI.WfB_sing, ``CoTickI.WfB_stream, ``CoTickI.embedSR_pair,
  ``CoTickI.embedSR_sing_sr, ``CoTickI.embedSR_sing_rr, ``CoTickI.embedSR_sing_wf,
  ``CoTickI.embedSR_stream_sr, ``CoTickI.embedSR_stream_wf, ``CoTickI.embedRR_pair,
  ``CoTickI.embedRR_sing_sr, ``CoTickI.embedRR_sing_rr, ``CoTickI.embedRR_sing_wf,
  ``CoTickI.embedRR_stream_rr, ``CoTickI.embedRR_stream_wf, ``CoTickI.embed2_pair,
  ``CoTickI.embed2_sing_sr, ``CoTickI.embed2_sing_rr, ``CoTickI.embed2_sing_wf,
  ``CoTickI.embed2_stream_sr, ``CoTickI.embed2_stream_rr, ``CoTickI.embed2_stream_coerce_wf,
  ``CoTickI.eq_idV_self, ``CoTickI.coerce_pair, ``CoTickI.coerce_sing,
  ``eq_self_iff_true, ``true_and, ``and_true,
  ``and_self, ``implies_true, ``ite_self,
  ``Prod.mk.injEq, ``co_bcount_wf, ``co_bfold_wf,
  ``co_bfirst_wf, ``co_bsPure_wf, ``co_bsMap_wf,
  ``co_bsZip_wf, ``co_boMap_wf, ``co_boUnwrapOr_wf,
  ``co_boFilter_wf, ``co_boIsSome_wf, ``co_timeout_snapshot_wf,
  ``co_source_interval_batch_wf, ``co_flattenOrdered_wf,
  ``co_flattenUnordered_wf, ``co_allTicks_wf,
  ``and_true, ``true_and]

/-- The `wf` normalizer: reduces a program output's residual `wf` to
its knots' components (all other ops thread conjunctively; boundary
inputs contribute `True`). -/
macro "co_wf_simp" "[" ids:Lean.Parser.Tactic.simpLemma,* "]" : tactic => do
  let rs ← coWfSimpRules.mapM fun n => `(Lean.Parser.Tactic.simpLemma| $(mkIdent n):ident)
  `(tactic| simp only [$ids,*, $rs,*])

end Hydro
