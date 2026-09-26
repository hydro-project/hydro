import HydroV2.Couple


/-!
# HydroV2 · coupling-corner projections (`CoupleProj`)

Per-op `rfl` projection lemmas for `CoupleSem` (the D39 corner): for
every op, its output's `rr` is the `Values` op at the self-derived
decision, its `sr` is the `SchedSem` op at the machine decision, and
its `wf` is the conjunction of its inputs' — proved once, at variable
arguments (never per program). `co_transfer [defs]` closes
`rr`/`sr`-naming identities by `simp only` + reducible `rfl`;
`co_wf [defs]` normalizes a program's residual `wf` to its knots'
components.
-/

namespace HydroV2

variable {L : Type} {mem : L → Nat}
  {pacing : (ℓ : L) → Fin (mem ℓ) → Nat → Bool} {Tc Td : Nat}
  {hjT : Tc ≤ Td} {ℓ : L}

/-! ## Boundary constructors (instance-projected types)

Program-facing spellings: the raw-record constructors of `Couple.lean`
restated at the instance-projected carrier types, so the projection
lemmas' keyed matching sees one spelling (the discipline established
by the retired `SquareProj.lean` — FINDINGS D33/D42). -/

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
    (s : (SchedSem L mem pacing).TickSingleton ℓ σ .unbounded)
    (v : (Values L mem).TickSingleton ℓ σ .unbounded)
    (hc : ∀ t i, s i t <+: v i) :
    (CoupleSem L mem pacing Tc Td hjT).TickSingleton ℓ σ .unbounded :=
  CoTickSing.input s v hc

/-- A coupled carrier from a single-horizon coupling (the graded
knot obligations construct these at lowered instances). -/
def CoStream.mkC {α : Type} [DecidableEq α] {ord : StrOrd}
    {ret : Retries} (x : (SchedSem L mem pacing).Stream ℓ α ord ret)
    (v : (Values L mem).Stream ℓ α ord ret)
    (hc : ∀ i, ListLe ord ret ((x i).view Tc) (v i)) :
    (CoupleSem L mem pacing Tc Td hjT).Stream ℓ α ord ret :=
  { sr := x, rr := v, wf := True, cpl := fun _ i => hc i }

def CoTickSing.mkC {σ : Type}
    (x : (SchedSem L mem pacing).TickSingleton ℓ σ .unbounded)
    (v : (Values L mem).TickSingleton ℓ σ .unbounded)
    (hc : ∀ k, k ≤ Tc → ∀ i, x i k <+: v i) :
    (CoupleSem L mem pacing Tc Td hjT).TickSingleton ℓ σ .unbounded :=
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
    (x : (SchedSem L mem pacing).TickSingleton ℓ σ .unbounded)
    (v : (Values L mem).TickSingleton ℓ σ .unbounded)
    (hc : ∀ k, k ≤ Tc → ∀ i, x i k <+: v i) :
    (CoTickSing.mkC (Td := Td) (hjT := hjT) x v hc).rr = v := rfl

theorem co_tick_mkC_sr {σ : Type}
    (x : (SchedSem L mem pacing).TickSingleton ℓ σ .unbounded)
    (v : (Values L mem).TickSingleton ℓ σ .unbounded)
    (hc : ∀ k, k ≤ Tc → ∀ i, x i k <+: v i) :
    (CoTickSing.mkC (Td := Td) (hjT := hjT) x v hc).sr = x := rfl

theorem co_tick_mkC_wf {σ : Type}
    (x : (SchedSem L mem pacing).TickSingleton ℓ σ .unbounded)
    (v : (Values L mem).TickSingleton ℓ σ .unbounded)
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
    (x : Fin n → Nat → Trace σ) (v : TickV n σ .unbounded) :
    (CoTickSing.probe2 (T := T) x v).rr = v := rfl

theorem co_tick_probe2_sr_raw {n : Nat} {σ : Type} {T : Nat}
    (x : Fin n → Nat → Trace σ) (v : TickV n σ .unbounded) :
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
    (x : Fin (mem ℓ) → Nat → Trace σ) (v : TickV (mem ℓ) σ .unbounded)
    (hc : ∀ k, k ≤ Tc → ∀ i, x i k <+: v i) :
    (CoTickSing.mkC (Td := Td) (hjT := hjT) (pacing := pacing)
      x v hc).rr = v := rfl

theorem co_tick_mkC_sr_raw {σ : Type}
    (x : Fin (mem ℓ) → Nat → Trace σ) (v : TickV (mem ℓ) σ .unbounded)
    (hc : ∀ k, k ≤ Tc → ∀ i, x i k <+: v i) :
    (CoTickSing.mkC (Td := Td) (hjT := hjT) (pacing := pacing)
      x v hc).sr = x := rfl

theorem co_tick_mkC_wf_raw {σ : Type}
    (x : Fin (mem ℓ) → Nat → Trace σ) (v : TickV (mem ℓ) σ .unbounded)
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
    (x : (SchedSem L mem pacing).TickSingleton ℓ σ .unbounded)
    (v : (Values L mem).TickSingleton ℓ σ .unbounded) :
    (CoupleSem L mem pacing Tc Td hjT).TickSingleton ℓ σ .unbounded :=
  CoTickSing.probe2 x v

/-- Machine/sched probe at the instance type. -/
def CoStream.schedC {α : Type} [DecidableEq α] {ord : StrOrd}
    {ret : Retries} (x : (SchedSem L mem pacing).Stream ℓ α ord ret) :
    (CoupleSem L mem pacing Tc Td hjT).Stream ℓ α ord ret :=
  CoStream.schedEmbed x

def CoTickSing.schedC {σ : Type}
    (x : (SchedSem L mem pacing).TickSingleton ℓ σ .unbounded) :
    (CoupleSem L mem pacing Tc Td hjT).TickSingleton ℓ σ .unbounded :=
  CoTickSing.schedEmbed x

/-- Horizon lowering at the instance types. -/
def CoStream.lowerC {α : Type} [DecidableEq α] {ord : StrOrd}
    {ret : Retries} {Tc' : Nat} (h' : Tc' ≤ Tc) (h'' : Tc' ≤ Td)
    (C : (CoupleSem L mem pacing Tc Td hjT).Stream ℓ α ord ret) :
    (CoupleSem L mem pacing Tc' Td h'').Stream ℓ α ord ret :=
  CoStream.lower C h'

def CoTickSing.lowerC {σ : Type} {Tc' : Nat} (h' : Tc' ≤ Tc)
    (h'' : Tc' ≤ Td)
    (C : (CoupleSem L mem pacing Tc Td hjT).TickSingleton ℓ σ
      .unbounded) :
    (CoupleSem L mem pacing Tc' Td h'').TickSingleton ℓ σ .unbounded :=
  CoTickSing.lower C h'

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

@[reducible] def CoDec.emit {n : Nat} {β : Type}
    (e : (SchedSem L mem pacing).EmitDec n β) :
    (CoupleSem L mem pacing Tc Td hjT).EmitDec n β := e

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
    (s : (SchedSem L mem pacing).TickSingleton ℓ σ .unbounded)
    (v : (Values L mem).TickSingleton ℓ σ .unbounded)
    (hc : ∀ t i, s i t <+: v i) :
    (CoTickSing.inputC (Tc := Tc) (Td := Td) (hjT := hjT)
      (pacing := pacing) s v hc).rr = v := rfl

theorem co_tick_input_sr {σ : Type}
    (s : (SchedSem L mem pacing).TickSingleton ℓ σ .unbounded)
    (v : (Values L mem).TickSingleton ℓ σ .unbounded)
    (hc : ∀ t i, s i t <+: v i) :
    (CoTickSing.inputC (Tc := Tc) (Td := Td) (hjT := hjT)
      (pacing := pacing) s v hc).sr = s := rfl

theorem co_tick_input_wf {σ : Type}
    (s : (SchedSem L mem pacing).TickSingleton ℓ σ .unbounded)
    (v : (Values L mem).TickSingleton ℓ σ .unbounded)
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
    (x : (SchedSem L mem pacing).TickSingleton ℓ σ .unbounded)
    (v : (Values L mem).TickSingleton ℓ σ .unbounded) :
    (CoTickSing.probeC (Tc := Tc) (Td := Td) (hjT := hjT) x v).rr = v
    := rfl

theorem co_tick_probeC_sr {σ : Type}
    (x : (SchedSem L mem pacing).TickSingleton ℓ σ .unbounded)
    (v : (Values L mem).TickSingleton ℓ σ .unbounded) :
    (CoTickSing.probeC (Tc := Tc) (Td := Td) (hjT := hjT) x v).sr = x
    := rfl

theorem co_schedC_sr {α : Type} [DecidableEq α]
    {ord : StrOrd} {ret : Retries}
    (x : (SchedSem L mem pacing).Stream ℓ α ord ret) :
    (CoStream.schedC (Tc := Tc) (Td := Td) (hjT := hjT) x).sr = x
    := rfl

theorem co_tick_schedC_sr {σ : Type}
    (x : (SchedSem L mem pacing).TickSingleton ℓ σ .unbounded) :
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

theorem co_tick_lowerC_rr {σ : Type} {Tc' : Nat} (h' : Tc' ≤ Tc)
    (h'' : Tc' ≤ Td)
    (C : (CoupleSem L mem pacing Tc Td hjT).TickSingleton ℓ σ
      .unbounded) :
    (CoTickSing.lowerC h' h'' C).rr = C.rr := rfl

theorem co_tick_lowerC_sr {σ : Type} {Tc' : Nat} (h' : Tc' ≤ Tc)
    (h'' : Tc' ≤ Td)
    (C : (CoupleSem L mem pacing Tc Td hjT).TickSingleton ℓ σ
      .unbounded) :
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

theorem co_broadcast_rr {c p : L} {α : Type} [DecidableEq α]
    {ord : StrOrd} {ret : Retries}
    (dt : (CoupleSem L mem pacing Tc Td hjT).TransportDec (mem p) (mem c))
    (s : (CoupleSem L mem pacing Tc Td hjT).Stream c α ord ret) :
    ((CoupleSem L mem pacing Tc Td hjT).broadcast (p := p) dt s).rr
      = (Values L mem).broadcast (p := p) () s.rr := rfl

theorem co_broadcast_sr {c p : L} {α : Type} [DecidableEq α]
    {ord : StrOrd} {ret : Retries}
    (dt : (CoupleSem L mem pacing Tc Td hjT).TransportDec (mem p) (mem c))
    (s : (CoupleSem L mem pacing Tc Td hjT).Stream c α ord ret) :
    ((CoupleSem L mem pacing Tc Td hjT).broadcast (p := p) dt s).sr
      = (SchedSem L mem pacing).broadcast (c := c) (p := p) (ord := ord)
          (ret := ret) dt s.sr := rfl

theorem co_broadcast_wf {c p : L} {α : Type} [DecidableEq α]
    {ord : StrOrd} {ret : Retries}
    (dt : (CoupleSem L mem pacing Tc Td hjT).TransportDec (mem p) (mem c))
    (s : (CoupleSem L mem pacing Tc Td hjT).Stream c α ord ret) :
    ((CoupleSem L mem pacing Tc Td hjT).broadcast (p := p) dt s).wf = s.wf
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

theorem co_mapBatchWith_rr {α σ β : Type} [DecidableEq α]
    (bs : (CoupleSem L mem pacing Tc Td hjT).TickStream ℓ α .totalOrder
      .exactlyOnce)
    (t : (CoupleSem L mem pacing Tc Td hjT).TickSingleton ℓ σ .unbounded)
    (f : Fin (mem ℓ) → List α → σ → β) :
    ((CoupleSem L mem pacing Tc Td hjT).mapBatchWith (ℓ := ℓ) bs t f).rr
      = (Values L mem).mapBatchWith (ℓ := ℓ) bs.rr t.rr f := rfl

theorem co_mapBatchWith_sr {α σ β : Type} [DecidableEq α]
    (bs : (CoupleSem L mem pacing Tc Td hjT).TickStream ℓ α .totalOrder
      .exactlyOnce)
    (t : (CoupleSem L mem pacing Tc Td hjT).TickSingleton ℓ σ .unbounded)
    (f : Fin (mem ℓ) → List α → σ → β) :
    ((CoupleSem L mem pacing Tc Td hjT).mapBatchWith (ℓ := ℓ) bs t f).sr
      = (SchedSem L mem pacing).mapBatchWith (ℓ := ℓ) bs.sr t.sr f
      := rfl

theorem co_mapBatchWith_wf {α σ β : Type} [DecidableEq α]
    (bs : (CoupleSem L mem pacing Tc Td hjT).TickStream ℓ α .totalOrder
      .exactlyOnce)
    (t : (CoupleSem L mem pacing Tc Td hjT).TickSingleton ℓ σ .unbounded)
    (f : Fin (mem ℓ) → List α → σ → β) :
    ((CoupleSem L mem pacing Tc Td hjT).mapBatchWith (ℓ := ℓ) bs t f).wf
      = (bs.wf ∧ t.wf) := rfl

theorem co_mapBatch_rr {α β : Type} [DecidableEq α]
    (bs : (CoupleSem L mem pacing Tc Td hjT).TickStream ℓ α .totalOrder
      .exactlyOnce)
    (f : Fin (mem ℓ) → List α → β) :
    ((CoupleSem L mem pacing Tc Td hjT).mapBatch (ℓ := ℓ) bs f).rr
      = (Values L mem).mapBatch (ℓ := ℓ) bs.rr f := rfl

theorem co_mapBatch_sr {α β : Type} [DecidableEq α]
    (bs : (CoupleSem L mem pacing Tc Td hjT).TickStream ℓ α .totalOrder
      .exactlyOnce)
    (f : Fin (mem ℓ) → List α → β) :
    ((CoupleSem L mem pacing Tc Td hjT).mapBatch (ℓ := ℓ) bs f).sr
      = (SchedSem L mem pacing).mapBatch (ℓ := ℓ) bs.sr f := rfl

theorem co_mapBatch_wf {α β : Type} [DecidableEq α]
    (bs : (CoupleSem L mem pacing Tc Td hjT).TickStream ℓ α .totalOrder
      .exactlyOnce)
    (f : Fin (mem ℓ) → List α → β) :
    ((CoupleSem L mem pacing Tc Td hjT).mapBatch (ℓ := ℓ) bs f).wf
      = bs.wf := rfl

theorem co_mapBatchesWith_rr {α σ β : Type} [DecidableEq α]
    [DecidableEq β]
    (bs : (CoupleSem L mem pacing Tc Td hjT).TickStream ℓ α .noOrder
      .exactlyOnce)
    (t : (CoupleSem L mem pacing Tc Td hjT).TickSingleton ℓ σ .unbounded)
    (f : Fin (mem ℓ) → α → σ → β) :
    ((CoupleSem L mem pacing Tc Td hjT).mapBatchesWith (ℓ := ℓ) bs t f).rr
      = (Values L mem).mapBatchesWith (ℓ := ℓ) bs.rr t.rr f
      := rfl

theorem co_mapBatchesWith_sr {α σ β : Type} [DecidableEq α]
    [DecidableEq β]
    (bs : (CoupleSem L mem pacing Tc Td hjT).TickStream ℓ α .noOrder
      .exactlyOnce)
    (t : (CoupleSem L mem pacing Tc Td hjT).TickSingleton ℓ σ .unbounded)
    (f : Fin (mem ℓ) → α → σ → β) :
    ((CoupleSem L mem pacing Tc Td hjT).mapBatchesWith (ℓ := ℓ) bs t f).sr
      = (SchedSem L mem pacing).mapBatchesWith (ℓ := ℓ) bs.sr t.sr f
      := rfl

theorem co_mapBatchesWith_wf {α σ β : Type} [DecidableEq α]
    [DecidableEq β]
    (bs : (CoupleSem L mem pacing Tc Td hjT).TickStream ℓ α .noOrder
      .exactlyOnce)
    (t : (CoupleSem L mem pacing Tc Td hjT).TickSingleton ℓ σ .unbounded)
    (f : Fin (mem ℓ) → α → σ → β) :
    ((CoupleSem L mem pacing Tc Td hjT).mapBatchesWith (ℓ := ℓ) bs t f).wf
      = (bs.wf ∧ t.wf) := rfl

theorem co_filterMapBatchesWith_rr {α σ β : Type}
    [DecidableEq α] [DecidableEq β]
    (bs : (CoupleSem L mem pacing Tc Td hjT).TickStream ℓ α .noOrder
      .exactlyOnce)
    (t : (CoupleSem L mem pacing Tc Td hjT).TickSingleton ℓ σ .unbounded)
    (f : Fin (mem ℓ) → α → σ → Option β) :
    ((CoupleSem L mem pacing Tc Td hjT).filterMapBatchesWith (ℓ := ℓ) bs t
        f).rr
      = (Values L mem).filterMapBatchesWith (ℓ := ℓ) bs.rr t.rr
          f := rfl

theorem co_filterMapBatchesWith_sr {α σ β : Type}
    [DecidableEq α] [DecidableEq β]
    (bs : (CoupleSem L mem pacing Tc Td hjT).TickStream ℓ α .noOrder
      .exactlyOnce)
    (t : (CoupleSem L mem pacing Tc Td hjT).TickSingleton ℓ σ .unbounded)
    (f : Fin (mem ℓ) → α → σ → Option β) :
    ((CoupleSem L mem pacing Tc Td hjT).filterMapBatchesWith (ℓ := ℓ) bs t f).sr
      = (SchedSem L mem pacing).filterMapBatchesWith (ℓ := ℓ) bs.sr t.sr
          f := rfl

theorem co_filterMapBatchesWith_wf {α σ β : Type}
    [DecidableEq α] [DecidableEq β]
    (bs : (CoupleSem L mem pacing Tc Td hjT).TickStream ℓ α .noOrder
      .exactlyOnce)
    (t : (CoupleSem L mem pacing Tc Td hjT).TickSingleton ℓ σ .unbounded)
    (f : Fin (mem ℓ) → α → σ → Option β) :
    ((CoupleSem L mem pacing Tc Td hjT).filterMapBatchesWith (ℓ := ℓ) bs t f).wf
      = (bs.wf ∧ t.wf) := rfl

theorem co_scan_batches_across_ticks_rr {α τ σ β : Type}
    [DecidableEq α]
    (bs : (CoupleSem L mem pacing Tc Td hjT).TickStream ℓ α .totalOrder
      .exactlyOnce)
    (t : (CoupleSem L mem pacing Tc Td hjT).TickSingleton ℓ τ .unbounded)
    (g : Fin (mem ℓ) → σ → List α → τ → σ × β) (init : σ) :
    ((CoupleSem L mem pacing Tc Td hjT).scan_batches_across_ticks (ℓ := ℓ) bs t
        g init).rr
      = (Values L mem).scan_batches_across_ticks (ℓ := ℓ) bs.rr
          t.rr g init := rfl

theorem co_scan_batches_across_ticks_sr {α τ σ β : Type}
    [DecidableEq α]
    (bs : (CoupleSem L mem pacing Tc Td hjT).TickStream ℓ α .totalOrder
      .exactlyOnce)
    (t : (CoupleSem L mem pacing Tc Td hjT).TickSingleton ℓ τ .unbounded)
    (g : Fin (mem ℓ) → σ → List α → τ → σ × β) (init : σ) :
    ((CoupleSem L mem pacing Tc Td hjT).scan_batches_across_ticks (ℓ := ℓ) bs t
        g init).sr
      = (SchedSem L mem pacing).scan_batches_across_ticks (ℓ := ℓ) bs.sr
          t.sr g init := rfl

theorem co_scan_batches_across_ticks_wf {α τ σ β : Type}
    [DecidableEq α]
    (bs : (CoupleSem L mem pacing Tc Td hjT).TickStream ℓ α .totalOrder
      .exactlyOnce)
    (t : (CoupleSem L mem pacing Tc Td hjT).TickSingleton ℓ τ .unbounded)
    (g : Fin (mem ℓ) → σ → List α → τ → σ × β) (init : σ) :
    ((CoupleSem L mem pacing Tc Td hjT).scan_batches_across_ticks (ℓ := ℓ) bs t
        g init).wf
      = (bs.wf ∧ t.wf) := rfl

theorem co_fold_batches_across_ticks_monotone_rr {α σ : Type}
    [DecidableEq α] (vo : ValueOrder σ) (g : Fin (mem ℓ) → σ → α → σ)
    (init : σ) (comm : ∀ i s x y, g i (g i s x) y = g i (g i s y) x)
    (hinfl : ∀ i s x, vo.le s (g i s x))
    (bs : (CoupleSem L mem pacing Tc Td hjT).TickStream ℓ α .noOrder
      .exactlyOnce) :
    ((CoupleSem L mem pacing Tc Td hjT).fold_batches_across_ticks_monotone
        (ℓ := ℓ) vo g init comm hinfl bs).rr
      = (Values L mem).fold_batches_across_ticks_monotone (ℓ := ℓ) vo g
          init comm hinfl bs.rr := rfl

theorem co_fold_batches_across_ticks_monotone_sr {α σ : Type}
    [DecidableEq α] (vo : ValueOrder σ) (g : Fin (mem ℓ) → σ → α → σ)
    (init : σ) (comm : ∀ i s x y, g i (g i s x) y = g i (g i s y) x)
    (hinfl : ∀ i s x, vo.le s (g i s x))
    (bs : (CoupleSem L mem pacing Tc Td hjT).TickStream ℓ α .noOrder
      .exactlyOnce) :
    ((CoupleSem L mem pacing Tc Td hjT).fold_batches_across_ticks_monotone
        (ℓ := ℓ) vo g init comm hinfl bs).sr
      = (SchedSem L mem pacing).fold_batches_across_ticks_monotone
          (ℓ := ℓ) vo g init comm hinfl bs.sr := rfl

theorem co_fold_batches_across_ticks_monotone_wf {α σ : Type}
    [DecidableEq α] (vo : ValueOrder σ) (g : Fin (mem ℓ) → σ → α → σ)
    (init : σ) (comm : ∀ i s x y, g i (g i s x) y = g i (g i s y) x)
    (hinfl : ∀ i s x, vo.le s (g i s x))
    (bs : (CoupleSem L mem pacing Tc Td hjT).TickStream ℓ α .noOrder
      .exactlyOnce) :
    ((CoupleSem L mem pacing Tc Td hjT).fold_batches_across_ticks_monotone
        (ℓ := ℓ) vo g init comm hinfl bs).wf
      = bs.wf := rfl

theorem co_scan_batches_unordered_across_ticks_rr
    {α τ σ β : Type} [DecidableEq α]
    (bs : (CoupleSem L mem pacing Tc Td hjT).TickStream ℓ α .noOrder
      .exactlyOnce)
    (t : (CoupleSem L mem pacing Tc Td hjT).TickSingleton ℓ τ .unbounded)
    (g : Fin (mem ℓ) → σ → Multiset α → τ → σ × β) (init : σ) :
    ((CoupleSem L mem pacing Tc Td hjT).scan_batches_unordered_across_ticks
        (ℓ := ℓ) bs t g init).rr
      = (Values L mem).scan_batches_unordered_across_ticks (ℓ := ℓ)
          bs.rr t.rr g init := rfl

theorem co_scan_batches_unordered_across_ticks_sr
    {α τ σ β : Type} [DecidableEq α]
    (bs : (CoupleSem L mem pacing Tc Td hjT).TickStream ℓ α .noOrder
      .exactlyOnce)
    (t : (CoupleSem L mem pacing Tc Td hjT).TickSingleton ℓ τ .unbounded)
    (g : Fin (mem ℓ) → σ → Multiset α → τ → σ × β) (init : σ) :
    ((CoupleSem L mem pacing Tc Td hjT).scan_batches_unordered_across_ticks
        (ℓ := ℓ) bs t g init).sr
      = (SchedSem L mem pacing).scan_batches_unordered_across_ticks
          (ℓ := ℓ) bs.sr t.sr g init := rfl

theorem co_scan_batches_unordered_across_ticks_wf
    {α τ σ β : Type} [DecidableEq α]
    (bs : (CoupleSem L mem pacing Tc Td hjT).TickStream ℓ α .noOrder
      .exactlyOnce)
    (t : (CoupleSem L mem pacing Tc Td hjT).TickSingleton ℓ τ .unbounded)
    (g : Fin (mem ℓ) → σ → Multiset α → τ → σ × β) (init : σ) :
    ((CoupleSem L mem pacing Tc Td hjT).scan_batches_unordered_across_ticks
        (ℓ := ℓ) bs t g init).wf
      = (bs.wf ∧ t.wf) := rfl

theorem co_scan_batches_unordered_rr {α σ β : Type}
    [DecidableEq α]
    (bs : (CoupleSem L mem pacing Tc Td hjT).TickStream ℓ α .noOrder
      .exactlyOnce)
    (g : Fin (mem ℓ) → σ → Multiset α → σ × β) (init : σ) :
    ((CoupleSem L mem pacing Tc Td hjT).scan_batches_unordered (ℓ := ℓ) bs g
        init).rr
      = (Values L mem).scan_batches_unordered (ℓ := ℓ) bs.rr g init
      := rfl

theorem co_scan_batches_unordered_sr {α σ β : Type}
    [DecidableEq α]
    (bs : (CoupleSem L mem pacing Tc Td hjT).TickStream ℓ α .noOrder
      .exactlyOnce)
    (g : Fin (mem ℓ) → σ → Multiset α → σ × β) (init : σ) :
    ((CoupleSem L mem pacing Tc Td hjT).scan_batches_unordered (ℓ := ℓ) bs g
        init).sr
      = (SchedSem L mem pacing).scan_batches_unordered (ℓ := ℓ) bs.sr g
          init := rfl

theorem co_scan_batches_unordered_wf {α σ β : Type}
    [DecidableEq α]
    (bs : (CoupleSem L mem pacing Tc Td hjT).TickStream ℓ α .noOrder
      .exactlyOnce)
    (g : Fin (mem ℓ) → σ → Multiset α → σ × β) (init : σ) :
    ((CoupleSem L mem pacing Tc Td hjT).scan_batches_unordered (ℓ := ℓ) bs g
        init).wf
      = bs.wf := rfl

theorem co_scan_batches_unordered₂_rr {α γ σ β : Type}
    [DecidableEq α] [DecidableEq γ]
    (bs : (CoupleSem L mem pacing Tc Td hjT).TickStream ℓ α .noOrder
      .exactlyOnce)
    (cs : (CoupleSem L mem pacing Tc Td hjT).TickStream ℓ γ .noOrder
      .exactlyOnce)
    (g : Fin (mem ℓ) → σ → Multiset α → Multiset γ → σ × β) (init : σ)
    :
    ((CoupleSem L mem pacing Tc Td hjT).scan_batches_unordered₂ (ℓ := ℓ) bs cs g
        init).rr
      = (Values L mem).scan_batches_unordered₂ (ℓ := ℓ) bs.rr
          cs.rr g init := rfl

theorem co_scan_batches_unordered₂_sr {α γ σ β : Type}
    [DecidableEq α] [DecidableEq γ]
    (bs : (CoupleSem L mem pacing Tc Td hjT).TickStream ℓ α .noOrder
      .exactlyOnce)
    (cs : (CoupleSem L mem pacing Tc Td hjT).TickStream ℓ γ .noOrder
      .exactlyOnce)
    (g : Fin (mem ℓ) → σ → Multiset α → Multiset γ → σ × β) (init : σ) :
    ((CoupleSem L mem pacing Tc Td hjT).scan_batches_unordered₂ (ℓ := ℓ) bs cs g
        init).sr
      = (SchedSem L mem pacing).scan_batches_unordered₂ (ℓ := ℓ) bs.sr
          cs.sr g init := rfl

theorem co_scan_batches_unordered₂_wf {α γ σ β : Type}
    [DecidableEq α] [DecidableEq γ]
    (bs : (CoupleSem L mem pacing Tc Td hjT).TickStream ℓ α .noOrder
      .exactlyOnce)
    (cs : (CoupleSem L mem pacing Tc Td hjT).TickStream ℓ γ .noOrder
      .exactlyOnce)
    (g : Fin (mem ℓ) → σ → Multiset α → Multiset γ → σ × β) (init : σ) :
    ((CoupleSem L mem pacing Tc Td hjT).scan_batches_unordered₂ (ℓ := ℓ) bs cs g
        init).wf
      = (bs.wf ∧ cs.wf) := rfl

theorem co_scan_across_ticks_rr {α σ β : Type}
    (t : (CoupleSem L mem pacing Tc Td hjT).TickSingleton ℓ α .unbounded)
    (g : Fin (mem ℓ) → σ → α → σ × β) (init : σ) :
    ((CoupleSem L mem pacing Tc Td hjT).scan_across_ticks (ℓ := ℓ) t g
        init).rr
      = (Values L mem).scan_across_ticks (ℓ := ℓ) t.rr g init := rfl

theorem co_scan_across_ticks_sr {α σ β : Type}
    (t : (CoupleSem L mem pacing Tc Td hjT).TickSingleton ℓ α .unbounded)
    (g : Fin (mem ℓ) → σ → α → σ × β) (init : σ) :
    ((CoupleSem L mem pacing Tc Td hjT).scan_across_ticks (ℓ := ℓ) t g init).sr
      = (SchedSem L mem pacing).scan_across_ticks (ℓ := ℓ) t.sr g init
      := rfl

theorem co_scan_across_ticks_wf {α σ β : Type}
    (t : (CoupleSem L mem pacing Tc Td hjT).TickSingleton ℓ α .unbounded)
    (g : Fin (mem ℓ) → σ → α → σ × β) (init : σ) :
    ((CoupleSem L mem pacing Tc Td hjT).scan_across_ticks (ℓ := ℓ) t g init).wf
      = t.wf := rfl

theorem co_mapTick_rr {α β : Type}
    (s : (CoupleSem L mem pacing Tc Td hjT).TickSingleton ℓ α .unbounded)
    (f : Fin (mem ℓ) → α → β) :
    ((CoupleSem L mem pacing Tc Td hjT).mapTick (ℓ := ℓ) s f).rr
      = (Values L mem).mapTick (ℓ := ℓ) s.rr f := rfl

theorem co_mapTick_sr {α β : Type}
    (s : (CoupleSem L mem pacing Tc Td hjT).TickSingleton ℓ α .unbounded)
    (f : Fin (mem ℓ) → α → β) :
    ((CoupleSem L mem pacing Tc Td hjT).mapTick (ℓ := ℓ) s f).sr
      = (SchedSem L mem pacing).mapTick (ℓ := ℓ) s.sr f := rfl

theorem co_mapTick_wf {α β : Type}
    (s : (CoupleSem L mem pacing Tc Td hjT).TickSingleton ℓ α .unbounded)
    (f : Fin (mem ℓ) → α → β) :
    ((CoupleSem L mem pacing Tc Td hjT).mapTick (ℓ := ℓ) s f).wf
      = s.wf := rfl

theorem co_zipTick_rr {α β : Type}
    (a : (CoupleSem L mem pacing Tc Td hjT).TickSingleton ℓ α .unbounded)
    (b : (CoupleSem L mem pacing Tc Td hjT).TickSingleton ℓ β .unbounded)
    :
    ((CoupleSem L mem pacing Tc Td hjT).zipTick (ℓ := ℓ) a b).rr
      = (Values L mem).zipTick (ℓ := ℓ) a.rr b.rr := rfl

theorem co_zipTick_sr {α β : Type}
    (a : (CoupleSem L mem pacing Tc Td hjT).TickSingleton ℓ α .unbounded)
    (b : (CoupleSem L mem pacing Tc Td hjT).TickSingleton ℓ β .unbounded) :
    ((CoupleSem L mem pacing Tc Td hjT).zipTick (ℓ := ℓ) a b).sr
      = (SchedSem L mem pacing).zipTick (ℓ := ℓ) a.sr b.sr := rfl

theorem co_zipTick_wf {α β : Type}
    (a : (CoupleSem L mem pacing Tc Td hjT).TickSingleton ℓ α .unbounded)
    (b : (CoupleSem L mem pacing Tc Td hjT).TickSingleton ℓ β .unbounded) :
    ((CoupleSem L mem pacing Tc Td hjT).zipTick (ℓ := ℓ) a b).wf
      = (a.wf ∧ b.wf) := rfl

theorem co_fold_across_ticks_monotone_rr {α σ : Type}
    (vo : ValueOrder σ) (g : Fin (mem ℓ) → σ → α → σ) (init : σ)
    (hinfl : ∀ i s x, vo.le s (g i s x))
    (s : (CoupleSem L mem pacing Tc Td hjT).TickSingleton ℓ α .unbounded)
    :
    ((CoupleSem L mem pacing Tc Td hjT).fold_across_ticks_monotone (ℓ := ℓ) vo g
        init hinfl s).rr
      = (Values L mem).fold_across_ticks_monotone (ℓ := ℓ) vo g init
          hinfl s.rr := rfl

theorem co_fold_across_ticks_monotone_sr {α σ : Type}
    (vo : ValueOrder σ) (g : Fin (mem ℓ) → σ → α → σ) (init : σ)
    (hinfl : ∀ i s x, vo.le s (g i s x))
    (s : (CoupleSem L mem pacing Tc Td hjT).TickSingleton ℓ α .unbounded) :
    ((CoupleSem L mem pacing Tc Td hjT).fold_across_ticks_monotone (ℓ := ℓ) vo g
        init hinfl s).sr
      = (SchedSem L mem pacing).fold_across_ticks_monotone (ℓ := ℓ) vo g
          init hinfl s.sr := rfl

theorem co_fold_across_ticks_monotone_wf {α σ : Type}
    (vo : ValueOrder σ) (g : Fin (mem ℓ) → σ → α → σ) (init : σ)
    (hinfl : ∀ i s x, vo.le s (g i s x))
    (s : (CoupleSem L mem pacing Tc Td hjT).TickSingleton ℓ α .unbounded) :
    ((CoupleSem L mem pacing Tc Td hjT).fold_across_ticks_monotone (ℓ := ℓ) vo g
        init hinfl s).wf
      = s.wf := rfl

theorem co_mapMonotone_rr {σ τ : Type} {vo : ValueOrder σ}
    (vo' : ValueOrder τ)
    (m : (CoupleSem L mem pacing Tc Td hjT).TickSingleton ℓ σ (.monotonic vo))
    (h : Fin (mem ℓ) → σ → τ)
    (hpres : ∀ i {a b}, vo.le a b → vo'.le (h i a) (h i b)) :
    ((CoupleSem L mem pacing Tc Td hjT).mapMonotone (ℓ := ℓ) (vo := vo) vo' m h
        hpres).rr
      = (Values L mem).mapMonotone (ℓ := ℓ) (vo := vo) vo' m.rr h
          hpres := rfl

theorem co_mapMonotone_sr {σ τ : Type} {vo : ValueOrder σ}
    (vo' : ValueOrder τ)
    (m : (CoupleSem L mem pacing Tc Td hjT).TickSingleton ℓ σ (.monotonic vo))
    (h : Fin (mem ℓ) → σ → τ)
    (hpres : ∀ i {a b}, vo.le a b → vo'.le (h i a) (h i b)) :
    ((CoupleSem L mem pacing Tc Td hjT).mapMonotone (ℓ := ℓ) (vo := vo) vo' m h
        hpres).sr
      = (SchedSem L mem pacing).mapMonotone (ℓ := ℓ) (vo := vo) vo' m.sr
          h hpres := rfl

theorem co_mapMonotone_wf {σ τ : Type} {vo : ValueOrder σ}
    (vo' : ValueOrder τ)
    (m : (CoupleSem L mem pacing Tc Td hjT).TickSingleton ℓ σ (.monotonic vo))
    (h : Fin (mem ℓ) → σ → τ)
    (hpres : ∀ i {a b}, vo.le a b → vo'.le (h i a) (h i b)) :
    ((CoupleSem L mem pacing Tc Td hjT).mapMonotone (ℓ := ℓ) (vo := vo) vo' m h
        hpres).wf
      = m.wf := rfl

theorem co_forgetBound_rr {σ : Type} {vo : ValueOrder σ}
    (m : (CoupleSem L mem pacing Tc Td hjT).TickSingleton ℓ σ (.monotonic vo))
    :
    ((CoupleSem L mem pacing Tc Td hjT).forgetBound (ℓ := ℓ) (vo := vo) m).rr
      = (Values L mem).forgetBound (ℓ := ℓ) (vo := vo) m.rr := rfl

theorem co_forgetBound_sr {σ : Type} {vo : ValueOrder σ}
    (m : (CoupleSem L mem pacing Tc Td hjT).TickSingleton ℓ σ (.monotonic vo)) :
    ((CoupleSem L mem pacing Tc Td hjT).forgetBound (ℓ := ℓ) (vo := vo) m).sr
      = (SchedSem L mem pacing).forgetBound (ℓ := ℓ) (vo := vo) m.sr
      := rfl

theorem co_forgetBound_wf {σ : Type} {vo : ValueOrder σ}
    (m : (CoupleSem L mem pacing Tc Td hjT).TickSingleton ℓ σ (.monotonic vo)) :
    ((CoupleSem L mem pacing Tc Td hjT).forgetBound (ℓ := ℓ) (vo := vo) m).wf
      = m.wf := rfl

theorem co_defer_rr {σ : Type} (init : σ)
    (t : (CoupleSem L mem pacing Tc Td hjT).TickSingleton ℓ σ .unbounded)
    :
    ((CoupleSem L mem pacing Tc Td hjT).defer (ℓ := ℓ) init t).rr
      = (Values L mem).defer (ℓ := ℓ) init t.rr := rfl

theorem co_defer_sr {σ : Type} (init : σ)
    (t : (CoupleSem L mem pacing Tc Td hjT).TickSingleton ℓ σ .unbounded) :
    ((CoupleSem L mem pacing Tc Td hjT).defer (ℓ := ℓ) init t).sr
      = (SchedSem L mem pacing).defer (ℓ := ℓ) init t.sr := rfl

theorem co_defer_wf {σ : Type} (init : σ)
    (t : (CoupleSem L mem pacing Tc Td hjT).TickSingleton ℓ σ .unbounded) :
    ((CoupleSem L mem pacing Tc Td hjT).defer (ℓ := ℓ) init t).wf
      = t.wf := rfl

theorem co_mapBatchesUnordered_rr {α σ β : Type}
    [DecidableEq α]
    (bs : (CoupleSem L mem pacing Tc Td hjT).TickStream ℓ α .noOrder
      .exactlyOnce)
    (t : (CoupleSem L mem pacing Tc Td hjT).TickSingleton ℓ σ .unbounded)
    (f : Fin (mem ℓ) → Multiset α → σ → β) :
    ((CoupleSem L mem pacing Tc Td hjT).mapBatchesUnordered (ℓ := ℓ) bs t
        f).rr
      = (Values L mem).mapBatchesUnordered (ℓ := ℓ) bs.rr t.rr f
      := rfl

theorem co_mapBatchesUnordered_sr {α σ β : Type}
    [DecidableEq α]
    (bs : (CoupleSem L mem pacing Tc Td hjT).TickStream ℓ α .noOrder
      .exactlyOnce)
    (t : (CoupleSem L mem pacing Tc Td hjT).TickSingleton ℓ σ .unbounded)
    (f : Fin (mem ℓ) → Multiset α → σ → β) :
    ((CoupleSem L mem pacing Tc Td hjT).mapBatchesUnordered (ℓ := ℓ) bs t f).sr
      = (SchedSem L mem pacing).mapBatchesUnordered (ℓ := ℓ) bs.sr t.sr
          f := rfl

theorem co_mapBatchesUnordered_wf {α σ β : Type}
    [DecidableEq α]
    (bs : (CoupleSem L mem pacing Tc Td hjT).TickStream ℓ α .noOrder
      .exactlyOnce)
    (t : (CoupleSem L mem pacing Tc Td hjT).TickSingleton ℓ σ .unbounded)
    (f : Fin (mem ℓ) → Multiset α → σ → β) :
    ((CoupleSem L mem pacing Tc Td hjT).mapBatchesUnordered (ℓ := ℓ) bs t f).wf
      = (bs.wf ∧ t.wf) := rfl

theorem co_emitBatches_rr {β : Type} [DecidableEq β]
    (t : (CoupleSem L mem pacing Tc Td hjT).TickSingleton ℓ (List β) .unbounded)
    :
    ((CoupleSem L mem pacing Tc Td hjT).emitBatches (ℓ := ℓ) t).rr
      = (Values L mem).emitBatches (ℓ := ℓ) t.rr := rfl

theorem co_emitBatches_sr {β : Type} [DecidableEq β]
    (t : (CoupleSem L mem pacing Tc Td hjT).TickSingleton ℓ (List β)
      .unbounded) :
    ((CoupleSem L mem pacing Tc Td hjT).emitBatches (ℓ := ℓ) t).sr
      = (SchedSem L mem pacing).emitBatches (ℓ := ℓ) t.sr := rfl

theorem co_emitBatches_wf {β : Type} [DecidableEq β]
    (t : (CoupleSem L mem pacing Tc Td hjT).TickSingleton ℓ (List β)
      .unbounded) :
    ((CoupleSem L mem pacing Tc Td hjT).emitBatches (ℓ := ℓ) t).wf
      = t.wf := rfl

theorem co_emitBatchesUnordered_rr {β : Type} [DecidableEq β]
    (t : (CoupleSem L mem pacing Tc Td hjT).TickSingleton ℓ (List β) .unbounded)
    :
    ((CoupleSem L mem pacing Tc Td hjT).emitBatchesUnordered (ℓ := ℓ) t).rr
      = (Values L mem).emitBatchesUnordered (ℓ := ℓ) t.rr := rfl

theorem co_emitBatchesUnordered_sr {β : Type} [DecidableEq β]
    (t : (CoupleSem L mem pacing Tc Td hjT).TickSingleton ℓ (List β)
      .unbounded) :
    ((CoupleSem L mem pacing Tc Td hjT).emitBatchesUnordered (ℓ := ℓ) t).sr
      = (SchedSem L mem pacing).emitBatchesUnordered (ℓ := ℓ) t.sr
      := rfl

theorem co_emitBatchesUnordered_wf {β : Type} [DecidableEq β]
    (t : (CoupleSem L mem pacing Tc Td hjT).TickSingleton ℓ (List β)
      .unbounded) :
    ((CoupleSem L mem pacing Tc Td hjT).emitBatchesUnordered (ℓ := ℓ) t).wf
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
    (t : (CoupleSem L mem pacing Tc Td hjT).TickSingleton ℓ (Option α)
      .unbounded)
    (times : (CoupleSem L mem pacing Tc Td hjT).SampleDec (mem ℓ)) :
    ((CoupleSem L mem pacing Tc Td hjT).sample_every t times).rr
      = (Values L mem).sample_every (ℓ := ℓ) t.rr times := rfl

theorem co_sample_every_sr {α : Type} [DecidableEq α]
    (t : (CoupleSem L mem pacing Tc Td hjT).TickSingleton ℓ (Option α)
      .unbounded)
    (times : (CoupleSem L mem pacing Tc Td hjT).SampleDec (mem ℓ)) :
    ((CoupleSem L mem pacing Tc Td hjT).sample_every t times).sr
      = (SchedSem L mem pacing).sample_every (ℓ := ℓ) t.sr times := rfl

theorem co_sample_every_wf {α : Type} [DecidableEq α]
    (t : (CoupleSem L mem pacing Tc Td hjT).TickSingleton ℓ (Option α)
      .unbounded)
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

theorem co_emitMultisetBatches_rr {β : Type} [DecidableEq β]
    (t : (CoupleSem L mem pacing Tc Td hjT).TickSingleton ℓ (Multiset β)
      .unbounded)
    (e : (CoupleSem L mem pacing Tc Td hjT).EmitDec (mem ℓ) β) :
    ((CoupleSem L mem pacing Tc Td hjT).emitMultisetBatches t e).rr
      = (Values L mem).emitMultisetBatches (ℓ := ℓ) t.rr () := rfl

theorem co_emitMultisetBatches_sr {β : Type} [DecidableEq β]
    (t : (CoupleSem L mem pacing Tc Td hjT).TickSingleton ℓ (Multiset β)
      .unbounded)
    (e : (CoupleSem L mem pacing Tc Td hjT).EmitDec (mem ℓ) β) :
    ((CoupleSem L mem pacing Tc Td hjT).emitMultisetBatches t e).sr
      = (SchedSem L mem pacing).emitMultisetBatches (ℓ := ℓ) t.sr e
    := rfl

theorem co_emitMultisetBatches_wf {β : Type} [DecidableEq β]
    (t : (CoupleSem L mem pacing Tc Td hjT).TickSingleton ℓ (Multiset β)
      .unbounded)
    (e : (CoupleSem L mem pacing Tc Td hjT).EmitDec (mem ℓ) β) :
    ((CoupleSem L mem pacing Tc Td hjT).emitMultisetBatches t e).wf = t.wf
    := rfl

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
    (body : ∀ H', Γ H' → H'.TickSingleton ℓ σ .unbounded →
      H'.TickSingleton ℓ σ .unbounded) :
    (SchedSem L mem pacing).TickSingleton ℓ σ .unbounded :=
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
    (body : ∀ H', Γ H' → H'.TickSingleton ℓ σ .unbounded →
      H'.TickSingleton ℓ σ .unbounded) :
    ((CoupleSem L mem pacing Tc Td hjT).fixTick (ℓ := ℓ) df caps
        body).rr
      = (Values L mem).fix_tick (ℓ := ℓ) (σ := σ) (Td + 1)
          (fun v => (body _ caps (CoTickSing.probeC
            (CoTickSing.fixSrC caps body) v)).rr) := rfl

theorem co_hfix_tick_sr {σ : Type}
    (df : (CoupleSem L mem pacing Tc Td hjT).FixDec)
    (caps : Γ (CoupleSem L mem pacing Tc Td hjT))
    (body : ∀ H', Γ H' → H'.TickSingleton ℓ σ .unbounded →
      H'.TickSingleton ℓ σ .unbounded) :
    ((CoupleSem L mem pacing Tc Td hjT).fixTick (ℓ := ℓ) df caps
        body).sr
      = CoTickSing.fixSrC caps body := rfl

theorem co_tick_fixSrC_eq {σ : Type}
    (caps : Γ (CoupleSem L mem pacing Tc Td hjT))
    (body : ∀ H', Γ H' → H'.TickSingleton ℓ σ .unbounded →
      H'.TickSingleton ℓ σ .unbounded) :
    CoTickSing.fixSrC caps body
      = (SchedSem L mem pacing).fix_tick (ℓ := ℓ) (σ := σ) ()
          (fun x => (body _ caps (CoTickSing.schedC x)).sr) := rfl

end FixProj

end Ops

/-! ## `HydroSem.fix` wrappers at the target instances (clean binders —
the retired `SquareProj.lean` copies captured an unused section
variable, which blocked `simp` from ever instantiating them) -/

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
    (body : ∀ H', Γ H' → H'.TickSingleton ℓ σ .unbounded →
      H'.TickSingleton ℓ σ .unbounded) :
    (Values L mem).fixTick (ℓ := ℓ) df caps body
      = (Values L mem).fix_tick (ℓ := ℓ) df
          (fun s => body _ caps s) := rfl

/-! ## Unit-decision normalizers (moved from the retired
`SquareProj.lean`; instance-generic — `Values`/`SchedSem` ops whose
decision type is `Unit` eta-reduce) -/

theorem values_broadcast_unit {α : Type} [DecidableEq α]
    {ord : StrOrd} {ret : Retries}
    (u : (Values L mem).TransportDec (mem p) (mem c))
    (s : (Values L mem).Stream c α ord ret) :
    (Values L mem).broadcast (c := c) (p := p) (ord := ord) (ret := ret)
        u s
      = (Values L mem).broadcast (c := c) (p := p) (ord := ord)
          (ret := ret) () s := rfl

theorem values_demux_unit {α : Type} [DecidableEq α]
    {ord : StrOrd} (u : (Values L mem).TransportDec (mem p) (mem c))
    (s : (Values L mem).Stream c (Nat × α) ord .exactlyOnce)
    (addr : Fin (mem p) → Nat) :
    (Values L mem).demux (c := c) (p := p) (ord := ord) u s addr
      = (Values L mem).demux (c := c) (p := p) (ord := ord) () s addr
      := rfl

theorem values_emitMultisetBatches_unit {β : Type}
    [DecidableEq β] (u : (Values L mem).EmitDec (mem ℓ) β)
    (t : (Values L mem).TickSingleton ℓ (Multiset β) .unbounded) :
    (Values L mem).emitMultisetBatches (ℓ := ℓ) t u
      = (Values L mem).emitMultisetBatches (ℓ := ℓ) t () := rfl

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

theorem sched_assume_ordering_batch_unit {α : Type}
    [DecidableEq α] (u : (SchedSem L mem pacing).BatchOrdSelDec (mem ℓ) α)
    (bs : (SchedSem L mem pacing).TickStream ℓ α .noOrder
      .exactlyOnce) :
    (SchedSem L mem pacing).assume_ordering_batch (ℓ := ℓ) bs u
      = (SchedSem L mem pacing).assume_ordering_batch (ℓ := ℓ) bs ()
      := rfl

theorem sched_fix_stream_unit {α : Type} [DecidableEq α]
    {ord : StrOrd} {ret : Retries}
    (u : (SchedSem L mem pacing).FixDec)
    (body : (SchedSem L mem pacing).Stream ℓ α ord ret →
      (SchedSem L mem pacing).Stream ℓ α ord ret) :
    (SchedSem L mem pacing).fix_stream (ℓ := ℓ) u body
      = (SchedSem L mem pacing).fix_stream (ℓ := ℓ) () body := rfl

theorem sched_fix_tick_unit {σ : Type}
    (u : (SchedSem L mem pacing).FixDec)
    (body : (SchedSem L mem pacing).TickSingleton ℓ σ .unbounded →
      (SchedSem L mem pacing).TickSingleton ℓ σ .unbounded) :
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
    (body : ∀ H', Γ H' → H'.TickSingleton ℓ σ .unbounded →
      H'.TickSingleton ℓ σ .unbounded) :
    (SchedSem L mem pacing).fixTick (ℓ := ℓ) df caps body
      = (SchedSem L mem pacing).fix_tick (ℓ := ℓ) df
          (fun s => body _ caps s) := rfl

end TargetFix

/-! ## The transfer macros -/

/-- The projection simp set: pushes `rr`/`sr` through every op. -/
macro "co_simp" "[" ids:Lean.Parser.Tactic.simpLemma,* "]" : tactic =>
  `(tactic| (
    simp only [$ids,*,
      co_input_rr, co_input_sr, co_tick_input_rr, co_tick_input_sr,
      co_probeC_rr, co_probeC_sr, co_tick_probeC_rr, co_tick_probeC_sr,
      co_mkC_rr, co_mkC_sr, co_tick_mkC_rr, co_tick_mkC_sr,
      co_mkC_rr_raw, co_mkC_sr_raw,
      co_tick_mkC_rr_raw, co_tick_mkC_sr_raw,
      co_embedS_sr_raw, co_tick_embedS_sr_raw,
      co_probe2_rr_raw, co_probe2_sr_raw,
      co_tick_probe2_rr_raw, co_tick_probe2_sr_raw,
      co_schedC_sr, co_tick_schedC_sr,
      co_lowerC_rr, co_lowerC_sr, co_tick_lowerC_rr, co_tick_lowerC_sr,
      co_map_rr, co_map_sr, co_filterMap_rr, co_filterMap_sr,
      co_broadcast_rr, co_broadcast_sr, co_demux_rr, co_demux_sr,
      co_values_rr, co_values_sr,
      co_weaken_retries_rr, co_weaken_retries_sr,
      co_union_rr, co_union_sr,
      co_assume_ordering_rr, co_assume_ordering_sr,
      co_fold_rr, co_fold_sr, co_fold_monotone_rr, co_fold_monotone_sr,
      co_snapshot_rr, co_snapshot_sr, co_batch_rr, co_batch_sr,
      co_batch_ordered_rr, co_batch_ordered_sr,
      co_mapBatchWith_rr, co_mapBatchWith_sr,
      co_mapBatch_rr, co_mapBatch_sr,
      co_mapBatchesWith_rr, co_mapBatchesWith_sr,
      co_filterMapBatchesWith_rr, co_filterMapBatchesWith_sr,
      co_scan_batches_across_ticks_rr, co_scan_batches_across_ticks_sr,
      co_fold_batches_across_ticks_monotone_rr,
      co_fold_batches_across_ticks_monotone_sr,
      co_scan_batches_unordered_across_ticks_rr,
      co_scan_batches_unordered_across_ticks_sr,
      co_scan_batches_unordered_rr, co_scan_batches_unordered_sr,
      co_scan_batches_unordered₂_rr, co_scan_batches_unordered₂_sr,
      co_scan_across_ticks_rr, co_scan_across_ticks_sr,
      co_mapTick_rr, co_mapTick_sr, co_zipTick_rr, co_zipTick_sr,
      co_fold_across_ticks_monotone_rr,
      co_fold_across_ticks_monotone_sr,
      co_mapMonotone_rr, co_mapMonotone_sr,
      co_forgetBound_rr, co_forgetBound_sr, co_defer_rr, co_defer_sr,
      co_sample_every_rr, co_sample_every_sr,
      co_timeout_snapshot_rr, co_timeout_snapshot_sr,
      co_source_interval_batch_rr, co_source_interval_batch_sr,
      co_mapBatchesUnordered_rr, co_mapBatchesUnordered_sr,
      co_emitBatches_rr, co_emitBatches_sr,
      co_emitMultisetBatches_rr, co_emitMultisetBatches_sr,
      co_emitBatchesUnordered_rr, co_emitBatchesUnordered_sr,
      co_allTicks_rr, co_allTicks_sr,
      co_hfix_stream_rr, co_hfix_stream_sr,
      co_hfix_tick_rr, co_hfix_tick_sr,
      values_hfix_stream', values_hfix_tick',
      sched_hfix_stream', sched_hfix_tick',
      values_broadcast_unit, values_demux_unit,
      values_emitMultisetBatches_unit,
      sched_assume_ordering_unit, sched_snapshot_unit, sched_batch_unit,
      sched_batch_ordered_unit, sched_assume_ordering_batch_unit,
      sched_fix_stream_unit, sched_fix_tick_unit]))

/-- The transfer identity closer: projections + reducible `rfl` (the
default-transparency fallback types the decision-record pins — witness
metavariables' raw-carrier assignments — after both sides are already
in simp-normal form). -/
macro "co_transfer" "[" ids:Lean.Parser.Tactic.simpLemma,* "]" :
    tactic =>
  `(tactic| (co_simp [$ids,*]; first
    | with_reducible rfl
    | exact rfl))

/-- The `wf` normalizer: reduces a program output's residual `wf` to
its knots' components (all other ops thread conjunctively; boundary
inputs contribute `True`). -/
macro "co_wf_simp" "[" ids:Lean.Parser.Tactic.simpLemma,* "]" :
    tactic =>
  `(tactic| (
    simp only [$ids,*,
      co_input_wf, co_tick_input_wf, co_mkC_wf, co_tick_mkC_wf,
      co_mkC_wf_raw, co_tick_mkC_wf_raw,
      co_map_wf, co_filterMap_wf, co_broadcast_wf, co_demux_wf,
      co_values_wf, co_weaken_retries_wf, co_union_wf,
      co_assume_ordering_wf, co_fold_wf, co_fold_monotone_wf,
      co_snapshot_wf, co_batch_wf, co_batch_ordered_wf,
      co_mapBatchWith_wf, co_mapBatch_wf, co_mapBatchesWith_wf,
      co_filterMapBatchesWith_wf, co_scan_batches_across_ticks_wf,
      co_fold_batches_across_ticks_monotone_wf,
      co_scan_batches_unordered_across_ticks_wf,
      co_scan_batches_unordered_wf, co_scan_batches_unordered₂_wf,
      co_scan_across_ticks_wf, co_mapTick_wf, co_zipTick_wf,
      co_fold_across_ticks_monotone_wf, co_mapMonotone_wf,
      co_forgetBound_wf, co_defer_wf, co_sample_every_wf,
      co_timeout_snapshot_wf, co_source_interval_batch_wf,
      co_mapBatchesUnordered_wf, co_emitBatches_wf,
      co_emitMultisetBatches_wf, co_emitBatchesUnordered_wf,
      co_allTicks_wf,
      and_true, true_and]))

end HydroV2
