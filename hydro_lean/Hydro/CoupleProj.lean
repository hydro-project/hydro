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
    (s : (SchedSem L mem pacing).Atomic ℓ σ)
    (v : (Values L mem).Atomic ℓ σ)
    (hc : ∀ t i, s i t <+: v i) (hm : TMono s) :
    (CoupleSem L mem pacing Tc Td hjT).Atomic ℓ σ :=
  CoTickSing.input s v hc hm

/-- An async-singleton input wire: both legs quote a common fold over
a source dominated by a common pool, at every horizon — the machine
leg is the fold's live value per step of the source `src` (the corner
keeps `src` to derive the read cuts), the denotational leg its read
function. The read-agreement package `CoSing.cpl` carries is derived
here once (`sched_snap_eq'` + `snapTrace_mono_dec`). -/
def CoSing.inputC {α σ : Type} [DecidableEq α] {ord : StrOrd}
    {ret : Retries} {b : SingBound σ}
    (h : (SchedSem L mem pacing).Singleton ℓ α σ ord ret b)
    (src : (SchedSem L mem pacing).Stream ℓ α ord ret)
    (v : (Values L mem).Singleton ℓ α σ ord ret b)
    (hc : ∃ (g : σ → α → σ) (init : σ) (ok : FoldOkP ord ret (fun _ _ => True) g)
      (pool : Fin (mem ℓ) → PoolCarrier α ord ret),
      (∀ i, singReads b v i = snapTrace ord ret (fun _ _ => True) g init ok (pool i)) ∧
      (∀ i t, h i t = ((src i).view t).foldl g init) ∧
      (∀ t i, ListLe ord ret ((src i).view t) (pool i))) :
    (CoupleSem L mem pacing Tc Td hjT).Singleton ℓ α σ ord ret b :=
  { sr := h, rr := v, src := src, wf := True,
    cpl := fun _ => by
      obtain ⟨g, init, ok, pool, hr, hf, hsrc⟩ := hc
      refine ⟨fun _i l => l.foldl g init, hf, ?_, ?_⟩
      · intro i ws hchain hpre
        rw [hr i]
        exact sched_snap_eq' ord ret _ g init ok (pool i) ws hchain
          (fun w hw => ListLe.of_prefix (hpre w hw) (hsrc Tc i)) (PoolPairwise_true _ _ _)
      · intro i d d' hdd
        rw [hr i]
        exact snapTrace_mono_dec _ g init ok (pool i) hdd }

/-- A coupled carrier from a single-horizon coupling (the graded
knot obligations construct these at lowered instances). -/
def CoStream.mkC {α : Type} [DecidableEq α] {ord : StrOrd}
    {ret : Retries} (x : (SchedSem L mem pacing).Stream ℓ α ord ret)
    (v : (Values L mem).Stream ℓ α ord ret)
    (hc : ∀ i, ListLe ord ret ((x i).view Tc) (v i)) :
    (CoupleSem L mem pacing Tc Td hjT).Stream ℓ α ord ret :=
  { sr := x, rr := v, wf := True, cpl := fun _ i => hc i }

def CoTickSing.mkC {σ : Type}
    (x : (SchedSem L mem pacing).Atomic ℓ σ)
    (v : (Values L mem).Atomic ℓ σ)
    (hc : ∀ k, k ≤ Tc → ∀ i, x i k <+: v i) (hm : TMono x) :
    (CoupleSem L mem pacing Tc Td hjT).Atomic ℓ σ :=
  { sr := x, rr := v, wf := True, cpl := fun _ k hk i => hc k hk i, mono := fun _ => hm }

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
    (x : (SchedSem L mem pacing).Atomic ℓ σ)
    (v : (Values L mem).Atomic ℓ σ)
    (hc : ∀ k, k ≤ Tc → ∀ i, x i k <+: v i) (hm : TMono x) :
    (CoTickSing.mkC (Td := Td) (hjT := hjT) x v hc hm).rr = v := rfl

theorem co_tick_mkC_sr {σ : Type}
    (x : (SchedSem L mem pacing).Atomic ℓ σ)
    (v : (Values L mem).Atomic ℓ σ)
    (hc : ∀ k, k ≤ Tc → ∀ i, x i k <+: v i) (hm : TMono x) :
    (CoTickSing.mkC (Td := Td) (hjT := hjT) x v hc hm).sr = x := rfl

theorem co_tick_mkC_wf {σ : Type}
    (x : (SchedSem L mem pacing).Atomic ℓ σ)
    (v : (Values L mem).Atomic ℓ σ)
    (hc : ∀ k, k ≤ Tc → ∀ i, x i k <+: v i) (hm : TMono x) :
    (CoTickSing.mkC (Td := Td) (hjT := hjT) x v hc hm).wf = True := rfl

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
    (v : MemberMap n (PoolCarrier α ord ret)) :
    (CoStream.probe2 (T := T) x v).rr = v := rfl

theorem co_probe2_sr_raw {n : Nat} {α : Type} [DecidableEq α]
    {ord : StrOrd} {ret : Retries} {T : Nat} (x : Fin n → StepHist α)
    (v : MemberMap n (PoolCarrier α ord ret)) :
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
    (v : MemberMap (mem ℓ) (PoolCarrier α ord ret))
    (hc : ∀ i, ListLe ord ret ((x i).view Tc) (v i)) :
    (CoStream.mkC (Td := Td) (hjT := hjT) (pacing := pacing)
      x v hc).rr = v := rfl

theorem co_mkC_sr_raw {α : Type} [DecidableEq α] {ord : StrOrd}
    {ret : Retries} (x : Fin (mem ℓ) → StepHist α)
    (v : MemberMap (mem ℓ) (PoolCarrier α ord ret))
    (hc : ∀ i, ListLe ord ret ((x i).view Tc) (v i)) :
    (CoStream.mkC (Td := Td) (hjT := hjT) (pacing := pacing)
      x v hc).sr = x := rfl

theorem co_mkC_wf_raw {α : Type} [DecidableEq α] {ord : StrOrd}
    {ret : Retries} (x : Fin (mem ℓ) → StepHist α)
    (v : MemberMap (mem ℓ) (PoolCarrier α ord ret))
    (hc : ∀ i, ListLe ord ret ((x i).view Tc) (v i)) :
    (CoStream.mkC (Td := Td) (hjT := hjT) (pacing := pacing)
      x v hc).wf = True := rfl

theorem co_tick_mkC_rr_raw {σ : Type}
    (x : Fin (mem ℓ) → Nat → Trace σ) (v : TickV (mem ℓ) σ)
    (hc : ∀ k, k ≤ Tc → ∀ i, x i k <+: v i) (hm : TMono x) :
    (CoTickSing.mkC (Td := Td) (hjT := hjT) (pacing := pacing)
      x v hc hm).rr = v := rfl

theorem co_tick_mkC_sr_raw {σ : Type}
    (x : Fin (mem ℓ) → Nat → Trace σ) (v : TickV (mem ℓ) σ)
    (hc : ∀ k, k ≤ Tc → ∀ i, x i k <+: v i) (hm : TMono x) :
    (CoTickSing.mkC (Td := Td) (hjT := hjT) (pacing := pacing)
      x v hc hm).sr = x := rfl

theorem co_tick_mkC_wf_raw {σ : Type}
    (x : Fin (mem ℓ) → Nat → Trace σ) (v : TickV (mem ℓ) σ)
    (hc : ∀ k, k ≤ Tc → ∀ i, x i k <+: v i) (hm : TMono x) :
    (CoTickSing.mkC (Td := Td) (hjT := hjT) (pacing := pacing)
      x v hc hm).wf = True := rfl

/-- The two-leg probe at the instance type. -/
def CoStream.probeC {α : Type} [DecidableEq α] {ord : StrOrd}
    {ret : Retries} (x : (SchedSem L mem pacing).Stream ℓ α ord ret)
    (v : (Values L mem).Stream ℓ α ord ret) :
    (CoupleSem L mem pacing Tc Td hjT).Stream ℓ α ord ret :=
  CoStream.probe2 x v

def CoTickSing.probeC {σ : Type}
    (x : (SchedSem L mem pacing).Atomic ℓ σ)
    (v : (Values L mem).Atomic ℓ σ) :
    (CoupleSem L mem pacing Tc Td hjT).Atomic ℓ σ :=
  CoTickSing.probe2 x v

/-- Machine/sched probe at the instance type. -/
def CoStream.schedC {α : Type} [DecidableEq α] {ord : StrOrd}
    {ret : Retries} (x : (SchedSem L mem pacing).Stream ℓ α ord ret) :
    (CoupleSem L mem pacing Tc Td hjT).Stream ℓ α ord ret :=
  CoStream.schedEmbed x

def CoTickSing.schedC {σ : Type}
    (x : (SchedSem L mem pacing).Atomic ℓ σ) :
    (CoupleSem L mem pacing Tc Td hjT).Atomic ℓ σ :=
  CoTickSing.schedEmbed x

/-- Horizon lowering at the instance types. -/
def CoStream.lowerC {α : Type} [DecidableEq α] {ord : StrOrd}
    {ret : Retries} {Tc' : Nat} (h' : Tc' ≤ Tc) (h'' : Tc' ≤ Td)
    (C : (CoupleSem L mem pacing Tc Td hjT).Stream ℓ α ord ret) :
    (CoupleSem L mem pacing Tc' Td h'').Stream ℓ α ord ret :=
  CoStream.lower C h'

def CoTickSing.lowerC {σ : Type} {Tc' : Nat} (h' : Tc' ≤ Tc)
    (h'' : Tc' ≤ Td)
    (C : (CoupleSem L mem pacing Tc Td hjT).Atomic ℓ σ) :
    (CoupleSem L mem pacing Tc' Td h'').Atomic ℓ σ :=
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

@[reducible] def CoDec.tickOrderSel {n : Nat} (α : Type) :
    (CoupleSem L mem pacing Tc Td hjT).TickOrderSelDec n α := ()

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

theorem co_sing_input_sr {α σ : Type} [DecidableEq α] {ord : StrOrd}
    {ret : Retries} {b : SingBound σ}
    (h : (SchedSem L mem pacing).Singleton ℓ α σ ord ret b)
    (src : (SchedSem L mem pacing).Stream ℓ α ord ret)
    (v : (Values L mem).Singleton ℓ α σ ord ret b) (hc) :
    (CoSing.inputC (Tc := Tc) (Td := Td) (hjT := hjT) h src v hc).sr = h := rfl

theorem co_sing_input_rr {α σ : Type} [DecidableEq α] {ord : StrOrd}
    {ret : Retries} {b : SingBound σ}
    (h : (SchedSem L mem pacing).Singleton ℓ α σ ord ret b)
    (src : (SchedSem L mem pacing).Stream ℓ α ord ret)
    (v : (Values L mem).Singleton ℓ α σ ord ret b) (hc) :
    (CoSing.inputC (Tc := Tc) (Td := Td) (hjT := hjT) h src v hc).rr = v := rfl

theorem co_sing_input_src {α σ : Type} [DecidableEq α] {ord : StrOrd}
    {ret : Retries} {b : SingBound σ}
    (h : (SchedSem L mem pacing).Singleton ℓ α σ ord ret b)
    (src : (SchedSem L mem pacing).Stream ℓ α ord ret)
    (v : (Values L mem).Singleton ℓ α σ ord ret b) (hc) :
    (CoSing.inputC (Tc := Tc) (Td := Td) (hjT := hjT) h src v hc).src = src := rfl

theorem co_tick_input_rr {σ : Type}
    (s : (SchedSem L mem pacing).Atomic ℓ σ)
    (v : (Values L mem).Atomic ℓ σ)
    (hc : ∀ t i, s i t <+: v i) (hm : TMono s) :
    (CoTickSing.inputC (Tc := Tc) (Td := Td) (hjT := hjT)
      (pacing := pacing) s v hc hm).rr = v := rfl

theorem co_tick_input_sr {σ : Type}
    (s : (SchedSem L mem pacing).Atomic ℓ σ)
    (v : (Values L mem).Atomic ℓ σ)
    (hc : ∀ t i, s i t <+: v i) (hm : TMono s) :
    (CoTickSing.inputC (Tc := Tc) (Td := Td) (hjT := hjT)
      (pacing := pacing) s v hc hm).sr = s := rfl

theorem co_tick_input_wf {σ : Type}
    (s : (SchedSem L mem pacing).Atomic ℓ σ)
    (v : (Values L mem).Atomic ℓ σ)
    (hc : ∀ t i, s i t <+: v i) (hm : TMono s) :
    (CoTickSing.inputC (Tc := Tc) (Td := Td) (hjT := hjT)
      (pacing := pacing) s v hc hm).wf = True := rfl

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
    (x : (SchedSem L mem pacing).Atomic ℓ σ)
    (v : (Values L mem).Atomic ℓ σ) :
    (CoTickSing.probeC (Tc := Tc) (Td := Td) (hjT := hjT) x v).rr = v
    := rfl

theorem co_tick_probeC_sr {σ : Type}
    (x : (SchedSem L mem pacing).Atomic ℓ σ)
    (v : (Values L mem).Atomic ℓ σ) :
    (CoTickSing.probeC (Tc := Tc) (Td := Td) (hjT := hjT) x v).sr = x
    := rfl

theorem co_schedC_sr {α : Type} [DecidableEq α]
    {ord : StrOrd} {ret : Retries}
    (x : (SchedSem L mem pacing).Stream ℓ α ord ret) :
    (CoStream.schedC (Tc := Tc) (Td := Td) (hjT := hjT) x).sr = x
    := rfl

theorem co_tick_schedC_sr {σ : Type}
    (x : (SchedSem L mem pacing).Atomic ℓ σ) :
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

theorem co_sing_lowerC_src {α σ : Type} [DecidableEq α]
    {ord : StrOrd} {ret : Retries} {b : SingBound σ} {Tc' : Nat}
    (h' : Tc' ≤ Tc) (h'' : Tc' ≤ Td)
    (C : (CoupleSem L mem pacing Tc Td hjT).Singleton ℓ α σ ord ret
      b) :
    (CoSing.lowerC h' h'' C).src = C.src := rfl

theorem co_tick_lowerC_rr {σ : Type} {Tc' : Nat} (h' : Tc' ≤ Tc)
    (h'' : Tc' ≤ Td)
    (C : (CoupleSem L mem pacing Tc Td hjT).Atomic ℓ σ) :
    (CoTickSing.lowerC h' h'' C).rr = C.rr := rfl

theorem co_tick_lowerC_sr {σ : Type} {Tc' : Nat} (h' : Tc' ≤ Tc)
    (h'' : Tc' ≤ Td)
    (C : (CoupleSem L mem pacing Tc Td hjT).Atomic ℓ σ) :
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

theorem co_values_rr {K V : Type} [DecidableEq K] [DecidableEq V]
    {ord : StrOrd} {ret : Retries}
    (k : (CoupleSem L mem pacing Tc Td hjT).KeyedStream ℓ K V ord ret) :
    ((CoupleSem L mem pacing Tc Td hjT).values k).rr
      = (Values L mem).values k.rr := rfl

theorem co_values_sr {K V : Type} [DecidableEq K] [DecidableEq V]
    {ord : StrOrd} {ret : Retries}
    (k : (CoupleSem L mem pacing Tc Td hjT).KeyedStream ℓ K V ord ret) :
    ((CoupleSem L mem pacing Tc Td hjT).values k).sr
      = (SchedSem L mem pacing).values (ℓ := ℓ) (ord := ord) (ret := ret) k.sr := rfl

theorem co_values_wf {K V : Type} [DecidableEq K] [DecidableEq V]
    {ord : StrOrd} {ret : Retries}
    (k : (CoupleSem L mem pacing Tc Td hjT).KeyedStream ℓ K V ord ret) :
    ((CoupleSem L mem pacing Tc Td hjT).values k).wf = k.wf := rfl

theorem co_into_keyed_rr {K V : Type} [DecidableEq K] [DecidableEq V] {ord : StrOrd}
    (s : (CoupleSem L mem pacing Tc Td hjT).Stream ℓ (K × V) ord .exactlyOnce) :
    ((CoupleSem L mem pacing Tc Td hjT).into_keyed s).rr = (Values L mem).into_keyed s.rr := rfl
theorem co_into_keyed_sr {K V : Type} [DecidableEq K] [DecidableEq V] {ord : StrOrd}
    (s : (CoupleSem L mem pacing Tc Td hjT).Stream ℓ (K × V) ord .exactlyOnce) :
    ((CoupleSem L mem pacing Tc Td hjT).into_keyed s).sr
      = (SchedSem L mem pacing).into_keyed (ℓ := ℓ) (ord := ord) s.sr := rfl
theorem co_into_keyed_wf {K V : Type} [DecidableEq K] [DecidableEq V] {ord : StrOrd}
    (s : (CoupleSem L mem pacing Tc Td hjT).Stream ℓ (K × V) ord .exactlyOnce) :
    ((CoupleSem L mem pacing Tc Td hjT).into_keyed s).wf = s.wf := rfl

theorem co_kassume_ordering_rr {K V : Type} [DecidableEq K] [DecidableEq V]
    (u : (CoupleSem L mem pacing Tc Td hjT).KeyedStream ℓ K V .noOrder .exactlyOnce)
    (sel : (CoupleSem L mem pacing Tc Td hjT).OrderSelDec (mem ℓ) (K × V)) :
    ((CoupleSem L mem pacing Tc Td hjT).kassume_ordering u sel).rr
      = (Values L mem).kassume_ordering u.rr (ordSelDerive Td u.sr) := rfl
theorem co_kassume_ordering_sr {K V : Type} [DecidableEq K] [DecidableEq V]
    (u : (CoupleSem L mem pacing Tc Td hjT).KeyedStream ℓ K V .noOrder .exactlyOnce)
    (sel : (CoupleSem L mem pacing Tc Td hjT).OrderSelDec (mem ℓ) (K × V)) :
    ((CoupleSem L mem pacing Tc Td hjT).kassume_ordering u sel).sr
      = (SchedSem L mem pacing).kassume_ordering (ℓ := ℓ) u.sr () := rfl
theorem co_kassume_ordering_wf {K V : Type} [DecidableEq K] [DecidableEq V]
    (u : (CoupleSem L mem pacing Tc Td hjT).KeyedStream ℓ K V .noOrder .exactlyOnce)
    (sel : (CoupleSem L mem pacing Tc Td hjT).OrderSelDec (mem ℓ) (K × V)) :
    ((CoupleSem L mem pacing Tc Td hjT).kassume_ordering u sel).wf = u.wf := rfl

theorem co_kfold_early_stop_rr {K V A : Type} [DecidableEq K] [DecidableEq V] [DecidableEq A]
    (ks : (CoupleSem L mem pacing Tc Td hjT).KeyedStream ℓ K V .totalOrder .exactlyOnce)
    (f : A → V → A × Bool) (init : A) :
    ((CoupleSem L mem pacing Tc Td hjT).kfold_early_stop ks f init).rr
      = (Values L mem).kfold_early_stop ks.rr f init := rfl
theorem co_kfold_early_stop_sr {K V A : Type} [DecidableEq K] [DecidableEq V] [DecidableEq A]
    (ks : (CoupleSem L mem pacing Tc Td hjT).KeyedStream ℓ K V .totalOrder .exactlyOnce)
    (f : A → V → A × Bool) (init : A) :
    ((CoupleSem L mem pacing Tc Td hjT).kfold_early_stop ks f init).sr
      = (SchedSem L mem pacing).kfold_early_stop (ℓ := ℓ) ks.sr f init := rfl
theorem co_kfold_early_stop_src {K V A : Type} [DecidableEq K] [DecidableEq V] [DecidableEq A]
    (ks : (CoupleSem L mem pacing Tc Td hjT).KeyedStream ℓ K V .totalOrder .exactlyOnce)
    (f : A → V → A × Bool) (init : A) :
    ((CoupleSem L mem pacing Tc Td hjT).kfold_early_stop ks f init).src = ks.sr := rfl
theorem co_kfold_early_stop_wf {K V A : Type} [DecidableEq K] [DecidableEq V] [DecidableEq A]
    (ks : (CoupleSem L mem pacing Tc Td hjT).KeyedStream ℓ K V .totalOrder .exactlyOnce)
    (f : A → V → A × Bool) (init : A) :
    ((CoupleSem L mem pacing Tc Td hjT).kfold_early_stop ks f init).wf = ks.wf := rfl

theorem co_kget_max_key_rr {α K V : Type} [DecidableEq α] [DecidableEq K] [DecidableEq V]
    [LinearOrder K] {ord : StrOrd} {ret : Retries}
    (ks : (CoupleSem L mem pacing Tc Td hjT).KeyedSingleton ℓ α K V ord ret .boundedValue) :
    ((CoupleSem L mem pacing Tc Td hjT).kget_max_key ks).rr
      = (Values L mem).kget_max_key (ord := ord) (ret := ret) ks.rr := rfl
theorem co_kget_max_key_sr {α K V : Type} [DecidableEq α] [DecidableEq K] [DecidableEq V]
    [LinearOrder K] {ord : StrOrd} {ret : Retries}
    (ks : (CoupleSem L mem pacing Tc Td hjT).KeyedSingleton ℓ α K V ord ret .boundedValue) :
    ((CoupleSem L mem pacing Tc Td hjT).kget_max_key ks).sr
      = (SchedSem L mem pacing).kget_max_key (ℓ := ℓ) (α := α) (ord := ord) (ret := ret) ks.sr := rfl
theorem co_kget_max_key_src {α K V : Type} [DecidableEq α] [DecidableEq K] [DecidableEq V]
    [LinearOrder K] {ord : StrOrd} {ret : Retries}
    (ks : (CoupleSem L mem pacing Tc Td hjT).KeyedSingleton ℓ α K V ord ret .boundedValue) :
    ((CoupleSem L mem pacing Tc Td hjT).kget_max_key ks).src = ks.src := rfl
theorem co_kget_max_key_wf {α K V : Type} [DecidableEq α] [DecidableEq K] [DecidableEq V]
    [LinearOrder K] {ord : StrOrd} {ret : Retries}
    (ks : (CoupleSem L mem pacing Tc Td hjT).KeyedSingleton ℓ α K V ord ret .boundedValue) :
    ((CoupleSem L mem pacing Tc Td hjT).kget_max_key ks).wf = ks.wf := rfl

theorem co_tinto_keyed_rr {K V : Type} [DecidableEq K] [DecidableEq V] {ord : StrOrd}
    (x : (CoupleSem L mem pacing Tc Td hjT).AtomicStream ℓ (K × V) ord .exactlyOnce) :
    ((CoupleSem L mem pacing Tc Td hjT).tinto_keyed x).rr = (Values L mem).tinto_keyed x.rr := rfl
theorem co_tinto_keyed_sr {K V : Type} [DecidableEq K] [DecidableEq V] {ord : StrOrd}
    (x : (CoupleSem L mem pacing Tc Td hjT).AtomicStream ℓ (K × V) ord .exactlyOnce) :
    ((CoupleSem L mem pacing Tc Td hjT).tinto_keyed x).sr
      = (SchedSem L mem pacing).tinto_keyed (ℓ := ℓ) (ord := ord) x.sr := rfl
theorem co_tinto_keyed_wf {K V : Type} [DecidableEq K] [DecidableEq V] {ord : StrOrd}
    (x : (CoupleSem L mem pacing Tc Td hjT).AtomicStream ℓ (K × V) ord .exactlyOnce) :
    ((CoupleSem L mem pacing Tc Td hjT).tinto_keyed x).wf = x.wf := rfl

theorem co_tassume_ordering_rr {α : Type} [DecidableEq α]
    (u : (CoupleSem L mem pacing Tc Td hjT).AtomicStream ℓ α .noOrder .exactlyOnce)
    (sel : (CoupleSem L mem pacing Tc Td hjT).TickOrderSelDec (mem ℓ) α) :
    ((CoupleSem L mem pacing Tc Td hjT).tassume_ordering u sel).rr
      = (Values L mem).tassume_ordering u.rr (tkOrdSelDerive Td u.sr) := rfl
theorem co_tassume_ordering_sr {α : Type} [DecidableEq α]
    (u : (CoupleSem L mem pacing Tc Td hjT).AtomicStream ℓ α .noOrder .exactlyOnce)
    (sel : (CoupleSem L mem pacing Tc Td hjT).TickOrderSelDec (mem ℓ) α) :
    ((CoupleSem L mem pacing Tc Td hjT).tassume_ordering u sel).sr
      = (SchedSem L mem pacing).tassume_ordering (ℓ := ℓ) u.sr () := rfl
theorem co_tassume_ordering_wf {α : Type} [DecidableEq α]
    (u : (CoupleSem L mem pacing Tc Td hjT).AtomicStream ℓ α .noOrder .exactlyOnce)
    (sel : (CoupleSem L mem pacing Tc Td hjT).TickOrderSelDec (mem ℓ) α) :
    ((CoupleSem L mem pacing Tc Td hjT).tassume_ordering u sel).wf = u.wf := rfl

theorem co_reduce_watermark_rr {K V : Type} [DecidableEq K] [DecidableEq V] [LinearOrder K]
    {ord : StrOrd}
    (ks : (CoupleSem L mem pacing Tc Td hjT).AtomicKeyedStream ℓ K V ord .exactlyOnce)
    (wm : (CoupleSem L mem pacing Tc Td hjT).Atomic ℓ (Option K)) (f : V → V → V)
    (Pre : V → V → Prop) [DecidableRel Pre] (ok : FoldOk ord .exactlyOnce Pre (reduceStep f)) :
    ((CoupleSem L mem pacing Tc Td hjT).reduce_watermark ks wm f Pre ok).rr
      = (Values L mem).reduce_watermark ks.rr wm.rr f Pre ok := rfl
theorem co_reduce_watermark_sr {K V : Type} [DecidableEq K] [DecidableEq V] [LinearOrder K]
    {ord : StrOrd}
    (ks : (CoupleSem L mem pacing Tc Td hjT).AtomicKeyedStream ℓ K V ord .exactlyOnce)
    (wm : (CoupleSem L mem pacing Tc Td hjT).Atomic ℓ (Option K)) (f : V → V → V)
    (Pre : V → V → Prop) [DecidableRel Pre] (ok : FoldOk ord .exactlyOnce Pre (reduceStep f)) :
    ((CoupleSem L mem pacing Tc Td hjT).reduce_watermark ks wm f Pre ok).sr
      = (SchedSem L mem pacing).reduce_watermark (ℓ := ℓ) ks.sr wm.sr f Pre ok := rfl
theorem co_reduce_watermark_wf {K V : Type} [DecidableEq K] [DecidableEq V] [LinearOrder K]
    {ord : StrOrd}
    (ks : (CoupleSem L mem pacing Tc Td hjT).AtomicKeyedStream ℓ K V ord .exactlyOnce)
    (wm : (CoupleSem L mem pacing Tc Td hjT).Atomic ℓ (Option K)) (f : V → V → V)
    (Pre : V → V → Prop) [DecidableRel Pre] (ok : FoldOk ord .exactlyOnce Pre (reduceStep f)) :
    ((CoupleSem L mem pacing Tc Td hjT).reduce_watermark ks wm f Pre ok).wf
      = (ks.wf ∧ wm.wf ∧ ∀ i, (KeyedPool.accum (ks.rr i)).Pairwise Pre) := rfl

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
    {ret : Retries} (g : σ → α → σ) (init : σ) (Pre : α → α → Prop) [DecidableRel Pre]
    (ok : FoldOk ord ret Pre g)
    (s : (CoupleSem L mem pacing Tc Td hjT).Stream ℓ α ord ret) :
    ((CoupleSem L mem pacing Tc Td hjT).fold g init Pre ok s).rr
      = (Values L mem).fold g init Pre ok s.rr := rfl

theorem co_fold_sr {α σ : Type} [DecidableEq α] {ord : StrOrd}
    {ret : Retries} (g : σ → α → σ) (init : σ) (Pre : α → α → Prop) [DecidableRel Pre]
    (ok : FoldOk ord ret Pre g)
    (s : (CoupleSem L mem pacing Tc Td hjT).Stream ℓ α ord ret) :
    ((CoupleSem L mem pacing Tc Td hjT).fold g init Pre ok s).sr
      = (SchedSem L mem pacing).fold (ℓ := ℓ) (ord := ord) (ret := ret)
          g init Pre ok s.sr := rfl

/-- The remembered machine source of a fold corner: the input's machine
leg (what `snapshot`'s derived cuts are computed from). -/
theorem co_fold_src {α σ : Type} [DecidableEq α] {ord : StrOrd}
    {ret : Retries} (g : σ → α → σ) (init : σ) (Pre : α → α → Prop) [DecidableRel Pre]
    (ok : FoldOk ord ret Pre g)
    (s : (CoupleSem L mem pacing Tc Td hjT).Stream ℓ α ord ret) :
    ((CoupleSem L mem pacing Tc Td hjT).fold g init Pre ok s).src = s.sr := rfl

theorem co_fold_wf {α σ : Type} [DecidableEq α] {ord : StrOrd}
    {ret : Retries} (g : σ → α → σ) (init : σ) (Pre : α → α → Prop) [DecidableRel Pre]
    (ok : FoldOk ord ret Pre g)
    (s : (CoupleSem L mem pacing Tc Td hjT).Stream ℓ α ord ret) :
    ((CoupleSem L mem pacing Tc Td hjT).fold g init Pre ok s).wf
      = (s.wf ∧ ∀ i, PoolPairwise ord ret Pre (s.rr i)) := rfl

theorem co_fold_monotone_rr {α σ : Type} [DecidableEq α]
    {ord : StrOrd} {ret : Retries} (vo : ValueOrder σ) (g : σ → α → σ)
    (init : σ) (ok : FoldOk ord ret (fun _ _ => True) g) (hinfl : ∀ s x, vo.le s (g s x))
    (s : (CoupleSem L mem pacing Tc Td hjT).Stream ℓ α ord ret) :
    ((CoupleSem L mem pacing Tc Td hjT).fold_monotone vo g init ok hinfl s).rr
      = (Values L mem).fold_monotone vo g init ok hinfl s.rr := rfl

theorem co_fold_monotone_sr {α σ : Type} [DecidableEq α]
    {ord : StrOrd} {ret : Retries} (vo : ValueOrder σ) (g : σ → α → σ)
    (init : σ) (ok : FoldOk ord ret (fun _ _ => True) g) (hinfl : ∀ s x, vo.le s (g s x))
    (s : (CoupleSem L mem pacing Tc Td hjT).Stream ℓ α ord ret) :
    ((CoupleSem L mem pacing Tc Td hjT).fold_monotone vo g init ok hinfl s).sr
      = (SchedSem L mem pacing).fold_monotone (ℓ := ℓ) (ord := ord)
          (ret := ret) vo g init ok hinfl s.sr := rfl

theorem co_fold_monotone_src {α σ : Type} [DecidableEq α]
    {ord : StrOrd} {ret : Retries} (vo : ValueOrder σ) (g : σ → α → σ)
    (init : σ) (ok : FoldOk ord ret (fun _ _ => True) g) (hinfl : ∀ s x, vo.le s (g s x))
    (s : (CoupleSem L mem pacing Tc Td hjT).Stream ℓ α ord ret) :
    ((CoupleSem L mem pacing Tc Td hjT).fold_monotone vo g init ok hinfl s).src
      = s.sr := rfl

theorem co_fold_monotone_wf {α σ : Type} [DecidableEq α]
    {ord : StrOrd} {ret : Retries} (vo : ValueOrder σ) (g : σ → α → σ)
    (init : σ) (ok : FoldOk ord ret (fun _ _ => True) g) (hinfl : ∀ s x, vo.le s (g s x))
    (s : (CoupleSem L mem pacing Tc Td hjT).Stream ℓ α ord ret) :
    ((CoupleSem L mem pacing Tc Td hjT).fold_monotone vo g init ok hinfl s).wf
      = s.wf := rfl

theorem co_snapshot_rr {α σ : Type} [DecidableEq α] {ord : StrOrd}
    {ret : Retries} {b : SingBound σ}
    (s : (CoupleSem L mem pacing Tc Td hjT).Singleton ℓ α σ ord ret b)
    (cutd : (CoupleSem L mem pacing Tc Td hjT).SnapDec (mem ℓ) α ord) :
    ((CoupleSem L mem pacing Tc Td hjT).snapshot s cutd).rr
      = (Values L mem).snapshot (ret := ret) s.rr
          (snapDerive ord ret (pacing ℓ) Td s.src) := rfl

theorem co_snapshot_sr {α σ : Type} [DecidableEq α] {ord : StrOrd}
    {ret : Retries} {b : SingBound σ}
    (s : (CoupleSem L mem pacing Tc Td hjT).Singleton ℓ α σ ord ret b)
    (cutd : (CoupleSem L mem pacing Tc Td hjT).SnapDec (mem ℓ) α ord) :
    ((CoupleSem L mem pacing Tc Td hjT).snapshot s cutd).sr
      = (SchedSem L mem pacing).snapshot (ℓ := ℓ) (α := α) (ord := ord)
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
    (s : (CoupleSem L mem pacing Tc Td hjT).Atomic ℓ α)
    (f : Fin (mem ℓ) → α → β) :
    ((CoupleSem L mem pacing Tc Td hjT).mapTick (ℓ := ℓ) s f).rr
      = (Values L mem).mapTick (ℓ := ℓ) s.rr f := rfl

theorem co_mapTick_sr {α β : Type}
    (s : (CoupleSem L mem pacing Tc Td hjT).Atomic ℓ α)
    (f : Fin (mem ℓ) → α → β) :
    ((CoupleSem L mem pacing Tc Td hjT).mapTick (ℓ := ℓ) s f).sr
      = (SchedSem L mem pacing).mapTick (ℓ := ℓ) s.sr f := rfl

theorem co_mapTick_wf {α β : Type}
    (s : (CoupleSem L mem pacing Tc Td hjT).Atomic ℓ α)
    (f : Fin (mem ℓ) → α → β) :
    ((CoupleSem L mem pacing Tc Td hjT).mapTick (ℓ := ℓ) s f).wf
      = s.wf := rfl

theorem co_zipTick_rr {α β : Type}
    (a : (CoupleSem L mem pacing Tc Td hjT).Atomic ℓ α)
    (b : (CoupleSem L mem pacing Tc Td hjT).Atomic ℓ β)
    :
    ((CoupleSem L mem pacing Tc Td hjT).zipTick (ℓ := ℓ) a b).rr
      = (Values L mem).zipTick (ℓ := ℓ) a.rr b.rr := rfl

theorem co_zipTick_sr {α β : Type}
    (a : (CoupleSem L mem pacing Tc Td hjT).Atomic ℓ α)
    (b : (CoupleSem L mem pacing Tc Td hjT).Atomic ℓ β) :
    ((CoupleSem L mem pacing Tc Td hjT).zipTick (ℓ := ℓ) a b).sr
      = (SchedSem L mem pacing).zipTick (ℓ := ℓ) a.sr b.sr := rfl

theorem co_zipTick_wf {α β : Type}
    (a : (CoupleSem L mem pacing Tc Td hjT).Atomic ℓ α)
    (b : (CoupleSem L mem pacing Tc Td hjT).Atomic ℓ β) :
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

theorem co_binto_keyed_rr {K V : Type} [DecidableEq K] [DecidableEq V]
    (b : (CoupleSem L mem pacing Tc Td hjT).BoundedStream (K × V) ord .exactlyOnce) :
    ((CoupleSem L mem pacing Tc Td hjT).binto_keyed b).rr = (Values L mem).binto_keyed b.rr := rfl
theorem co_binto_keyed_sr {K V : Type} [DecidableEq K] [DecidableEq V]
    (b : (CoupleSem L mem pacing Tc Td hjT).BoundedStream (K × V) ord .exactlyOnce) :
    ((CoupleSem L mem pacing Tc Td hjT).binto_keyed b).sr
      = (SchedSem L mem pacing).binto_keyed (ord := ord) b.sr := rfl
theorem co_binto_keyed_wf {K V : Type} [DecidableEq K] [DecidableEq V]
    (b : (CoupleSem L mem pacing Tc Td hjT).BoundedStream (K × V) ord .exactlyOnce) :
    ((CoupleSem L mem pacing Tc Td hjT).binto_keyed b).wf = b.wf := rfl

theorem co_bkfold_rr {K V A : Type} [DecidableEq K] [DecidableEq V] [DecidableEq A]
    (g : A → V → A) (init : A) (Pre : V → V → Prop) [DecidableRel Pre] (ok : FoldOk ord ret Pre g)
    (ks : (CoupleSem L mem pacing Tc Td hjT).BoundedKeyedStream K V ord ret) :
    ((CoupleSem L mem pacing Tc Td hjT).bkfold g init Pre ok ks).rr
      = (Values L mem).bkfold g init Pre ok ks.rr := rfl
theorem co_bkfold_sr {K V A : Type} [DecidableEq K] [DecidableEq V] [DecidableEq A]
    (g : A → V → A) (init : A) (Pre : V → V → Prop) [DecidableRel Pre] (ok : FoldOk ord ret Pre g)
    (ks : (CoupleSem L mem pacing Tc Td hjT).BoundedKeyedStream K V ord ret) :
    ((CoupleSem L mem pacing Tc Td hjT).bkfold g init Pre ok ks).sr
      = (SchedSem L mem pacing).bkfold (ord := ord) (ret := ret) g init Pre ok ks.sr := rfl
theorem co_bkfold_wf {K V A : Type} [DecidableEq K] [DecidableEq V] [DecidableEq A]
    (g : A → V → A) (init : A) (Pre : V → V → Prop) [DecidableRel Pre] (ok : FoldOk ord ret Pre g)
    (ks : (CoupleSem L mem pacing Tc Td hjT).BoundedKeyedStream K V ord ret) :
    ((CoupleSem L mem pacing Tc Td hjT).bkfold g init Pre ok ks).wf
      = (ks.wf ∧ ret = .exactlyOnce ∧ ks.rr.Pairwise Pre) := rfl

theorem co_bksFilter_rr {K V : Type} [DecidableEq K] [DecidableEq V]
    (m : (CoupleSem L mem pacing Tc Td hjT).BoundedKeyedSingleton K V) (p : V → Bool) :
    ((CoupleSem L mem pacing Tc Td hjT).bksFilter m p).rr = (Values L mem).bksFilter m.rr p := rfl
theorem co_bksFilter_sr {K V : Type} [DecidableEq K] [DecidableEq V]
    (m : (CoupleSem L mem pacing Tc Td hjT).BoundedKeyedSingleton K V) (p : V → Bool) :
    ((CoupleSem L mem pacing Tc Td hjT).bksFilter m p).sr = (SchedSem L mem pacing).bksFilter m.sr p := rfl
theorem co_bksFilter_wf {K V : Type} [DecidableEq K] [DecidableEq V]
    (m : (CoupleSem L mem pacing Tc Td hjT).BoundedKeyedSingleton K V) (p : V → Bool) :
    ((CoupleSem L mem pacing Tc Td hjT).bksFilter m p).wf = m.wf := rfl

theorem co_bksMap_rr {K V W : Type} [DecidableEq K] [DecidableEq V] [DecidableEq W]
    (m : (CoupleSem L mem pacing Tc Td hjT).BoundedKeyedSingleton K V) (f : V → W) :
    ((CoupleSem L mem pacing Tc Td hjT).bksMap m f).rr = (Values L mem).bksMap m.rr f := rfl
theorem co_bksMap_sr {K V W : Type} [DecidableEq K] [DecidableEq V] [DecidableEq W]
    (m : (CoupleSem L mem pacing Tc Td hjT).BoundedKeyedSingleton K V) (f : V → W) :
    ((CoupleSem L mem pacing Tc Td hjT).bksMap m f).sr = (SchedSem L mem pacing).bksMap m.sr f := rfl
theorem co_bksMap_wf {K V W : Type} [DecidableEq K] [DecidableEq V] [DecidableEq W]
    (m : (CoupleSem L mem pacing Tc Td hjT).BoundedKeyedSingleton K V) (f : V → W) :
    ((CoupleSem L mem pacing Tc Td hjT).bksMap m f).wf = m.wf := rfl

theorem co_bksKeys_rr {K V : Type} [DecidableEq K] [DecidableEq V]
    (m : (CoupleSem L mem pacing Tc Td hjT).BoundedKeyedSingleton K V) :
    ((CoupleSem L mem pacing Tc Td hjT).bksKeys m).rr = (Values L mem).bksKeys m.rr := rfl
theorem co_bksKeys_sr {K V : Type} [DecidableEq K] [DecidableEq V]
    (m : (CoupleSem L mem pacing Tc Td hjT).BoundedKeyedSingleton K V) :
    ((CoupleSem L mem pacing Tc Td hjT).bksKeys m).sr = (SchedSem L mem pacing).bksKeys m.sr := rfl
theorem co_bksKeys_wf {K V : Type} [DecidableEq K] [DecidableEq V]
    (m : (CoupleSem L mem pacing Tc Td hjT).BoundedKeyedSingleton K V) :
    ((CoupleSem L mem pacing Tc Td hjT).bksKeys m).wf = m.wf := rfl

theorem co_bksEntries_rr {K V : Type} [DecidableEq K] [DecidableEq V]
    (m : (CoupleSem L mem pacing Tc Td hjT).BoundedKeyedSingleton K V) :
    ((CoupleSem L mem pacing Tc Td hjT).bksEntries m).rr = (Values L mem).bksEntries m.rr := rfl
theorem co_bksEntries_sr {K V : Type} [DecidableEq K] [DecidableEq V]
    (m : (CoupleSem L mem pacing Tc Td hjT).BoundedKeyedSingleton K V) :
    ((CoupleSem L mem pacing Tc Td hjT).bksEntries m).sr = (SchedSem L mem pacing).bksEntries m.sr := rfl
theorem co_bksEntries_wf {K V : Type} [DecidableEq K] [DecidableEq V]
    (m : (CoupleSem L mem pacing Tc Td hjT).BoundedKeyedSingleton K V) :
    ((CoupleSem L mem pacing Tc Td hjT).bksEntries m).wf = m.wf := rfl
/-- **The structural closer for a `.entries().fold(HashMap::new, insert)`**
(paxos.rs:866–873's `commutative = manual_proof!(/** no keys will
overlap */)`): the pairwise conjunct a conditional in-tick fold over a
map's entries leaves in `wf` is a fact about every map — closed by the
wf normalizer, no `ghost pre` needed. -/
theorem co_bksEntries_rr_pairwise_key {K V : Type} [DecidableEq K] [DecidableEq V]
    (m : (CoupleSem L mem pacing Tc Td hjT).BoundedKeyedSingleton K V) :
    PoolPairwise .noOrder .exactlyOnce NoKeyOverlap
      ((CoupleSem L mem pacing Tc Td hjT).bksEntries m).rr :=
  KeyedMap.entries_pairwise_key m.rr

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
    (Pre : α → α → Prop) [DecidableRel Pre] (ok : FoldOk ord ret Pre g)
    (b : (CoupleSem L mem pacing Tc Td hjT).BoundedStream α ord ret) :
    ((CoupleSem L mem pacing Tc Td hjT).bfold g init Pre ok b).rr
      = (Values L mem).bfold g init Pre ok b.rr := rfl
theorem co_bfold_sr {ret : Retries} (g : σ → α → σ) (init : σ)
    (Pre : α → α → Prop) [DecidableRel Pre] (ok : FoldOk ord ret Pre g)
    (b : (CoupleSem L mem pacing Tc Td hjT).BoundedStream α ord ret) :
    ((CoupleSem L mem pacing Tc Td hjT).bfold g init Pre ok b).sr
      = (SchedSem L mem pacing).bfold g init Pre ok b.sr := rfl
theorem co_bfold_wf {ret : Retries} (g : σ → α → σ) (init : σ)
    (Pre : α → α → Prop) [DecidableRel Pre] (ok : FoldOk ord ret Pre g)
    (b : (CoupleSem L mem pacing Tc Td hjT).BoundedStream α ord ret) :
    ((CoupleSem L mem pacing Tc Td hjT).bfold g init Pre ok b).wf
      = (b.wf ∧ ret = .exactlyOnce ∧ PoolPairwise ord ret Pre b.rr) := rfl

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

def srLegs (sh : TickShape) (x : AtomicOf (CoupleSem L mem pacing Tc Td hjT).Atomic (CoupleSem L mem pacing Tc Td hjT).AtomicStream (CoupleSem L mem pacing Tc Td hjT).AtomicKeyedSingleton ℓ (sh)) : AtomicOf (SchedSem L mem pacing).Atomic (SchedSem L mem pacing).AtomicStream (SchedSem L mem pacing).AtomicKeyedSingleton ℓ (sh) := CoTick.srLegs sh x
def rrLegs (sh : TickShape) (x : AtomicOf (CoupleSem L mem pacing Tc Td hjT).Atomic (CoupleSem L mem pacing Tc Td hjT).AtomicStream (CoupleSem L mem pacing Tc Td hjT).AtomicKeyedSingleton ℓ (sh)) : AtomicOf (Values L mem).Atomic (Values L mem).AtomicStream (Values L mem).AtomicKeyedSingleton ℓ (sh) := CoTick.rrLegs sh x
def Wf (sh : TickShape) (x : AtomicOf (CoupleSem L mem pacing Tc Td hjT).Atomic (CoupleSem L mem pacing Tc Td hjT).AtomicStream (CoupleSem L mem pacing Tc Td hjT).AtomicKeyedSingleton ℓ (sh)) : Prop := CoTick.Wf sh x
def mk (sh : TickShape) (m : AtomicOf (SchedSem L mem pacing).Atomic (SchedSem L mem pacing).AtomicStream (SchedSem L mem pacing).AtomicKeyedSingleton ℓ (sh)) (v : AtomicOf (Values L mem).Atomic (Values L mem).AtomicStream (Values L mem).AtomicKeyedSingleton ℓ (sh)) (wf : Prop)
    (h : wf → ∀ k, k ≤ Tc → CorrTick.Le sh m k v) (hm : wf → TMonoOf sh m) : AtomicOf (CoupleSem L mem pacing Tc Td hjT).Atomic (CoupleSem L mem pacing Tc Td hjT).AtomicStream (CoupleSem L mem pacing Tc Td hjT).AtomicKeyedSingleton ℓ (sh) :=
  CoTick.mk sh m v wf h hm
def projSR (sh : TickShape) (x : BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (CoupleSem L mem pacing Tc Td hjT).BoundedKeyedSingleton (sh)) : BoundedOf (SchedSem L mem pacing).BoundedSingleton (SchedSem L mem pacing).BoundedStream (SchedSem L mem pacing).BoundedKeyedSingleton (sh) := CoTick.projSR sh x
def projRR (sh : TickShape) (x : BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (CoupleSem L mem pacing Tc Td hjT).BoundedKeyedSingleton (sh)) : BoundedOf (Values L mem).BoundedSingleton (Values L mem).BoundedStream (Values L mem).BoundedKeyedSingleton (sh) := CoTick.projRR sh x
def WfB (sh : TickShape) (x : BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (CoupleSem L mem pacing Tc Td hjT).BoundedKeyedSingleton (sh)) : Prop := CoTick.WfB sh x
def embedSR (sh : TickShape) (m : BoundedOf (SchedSem L mem pacing).BoundedSingleton (SchedSem L mem pacing).BoundedStream (SchedSem L mem pacing).BoundedKeyedSingleton (sh)) : BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (CoupleSem L mem pacing Tc Td hjT).BoundedKeyedSingleton (sh) := CoTick.embedSR sh m
def embedRR (sh : TickShape) (v : BoundedOf (Values L mem).BoundedSingleton (Values L mem).BoundedStream (Values L mem).BoundedKeyedSingleton (sh)) : BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (CoupleSem L mem pacing Tc Td hjT).BoundedKeyedSingleton (sh) := CoTick.embedRR sh v
def embed2 (sh : TickShape) (m : BoundedOf (SchedSem L mem pacing).BoundedSingleton (SchedSem L mem pacing).BoundedStream (SchedSem L mem pacing).BoundedKeyedSingleton (sh)) (v : BoundedOf (Values L mem).BoundedSingleton (Values L mem).BoundedStream (Values L mem).BoundedKeyedSingleton (sh)) : BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (CoupleSem L mem pacing Tc Td hjT).BoundedKeyedSingleton (sh) :=
  CoTick.embed2 sh m v
def coerce (sh : TickShape) (m : BoundedOf (SchedSem L mem pacing).BoundedSingleton (SchedSem L mem pacing).BoundedStream (SchedSem L mem pacing).BoundedKeyedSingleton (sh)) : BoundedOf (Values L mem).BoundedSingleton (Values L mem).BoundedStream (Values L mem).BoundedKeyedSingleton (sh) := CoTick.coerce sh m
/-- The machine's singleton value, presented at the denotation's type
(definitionally the identity; keeps the walk's terms consistently
typed so the leaf rules unify). -/
abbrev idV (v : (SchedSem L mem pacing).BoundedSingleton σ) : (Values L mem).BoundedSingleton σ := v
def gS (g : Fin (mem ℓ) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (CoupleSem L mem pacing Tc Td hjT).BoundedKeyedSingleton (sts) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (CoupleSem L mem pacing Tc Td hjT).BoundedKeyedSingleton (ins) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (CoupleSem L mem pacing Tc Td hjT).BoundedKeyedSingleton (sts) × BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (CoupleSem L mem pacing Tc Td hjT).BoundedKeyedSingleton (outs)) : Fin (mem ℓ) → BoundedOf (SchedSem L mem pacing).BoundedSingleton (SchedSem L mem pacing).BoundedStream (SchedSem L mem pacing).BoundedKeyedSingleton (sts) → BoundedOf (SchedSem L mem pacing).BoundedSingleton (SchedSem L mem pacing).BoundedStream (SchedSem L mem pacing).BoundedKeyedSingleton (ins) →
    BoundedOf (SchedSem L mem pacing).BoundedSingleton (SchedSem L mem pacing).BoundedStream (SchedSem L mem pacing).BoundedKeyedSingleton (sts) × BoundedOf (SchedSem L mem pacing).BoundedSingleton (SchedSem L mem pacing).BoundedStream (SchedSem L mem pacing).BoundedKeyedSingleton (outs) :=
  fun i s inp => @Prod.mk (BoundedOf (SchedSem L mem pacing).BoundedSingleton (SchedSem L mem pacing).BoundedStream (SchedSem L mem pacing).BoundedKeyedSingleton (sts))
    (BoundedOf (SchedSem L mem pacing).BoundedSingleton (SchedSem L mem pacing).BoundedStream (SchedSem L mem pacing).BoundedKeyedSingleton outs)
    (projSR sts (g i (embedSR sts s) (embedSR ins inp)).1)
    (projSR outs (g i (embedSR sts s) (embedSR ins inp)).2)
def gR (g : Fin (mem ℓ) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (CoupleSem L mem pacing Tc Td hjT).BoundedKeyedSingleton (sts) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (CoupleSem L mem pacing Tc Td hjT).BoundedKeyedSingleton (ins) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (CoupleSem L mem pacing Tc Td hjT).BoundedKeyedSingleton (sts) × BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (CoupleSem L mem pacing Tc Td hjT).BoundedKeyedSingleton (outs)) : Fin (mem ℓ) → BoundedOf (Values L mem).BoundedSingleton (Values L mem).BoundedStream (Values L mem).BoundedKeyedSingleton (sts) → BoundedOf (Values L mem).BoundedSingleton (Values L mem).BoundedStream (Values L mem).BoundedKeyedSingleton (ins) →
    BoundedOf (Values L mem).BoundedSingleton (Values L mem).BoundedStream (Values L mem).BoundedKeyedSingleton (sts) × BoundedOf (Values L mem).BoundedSingleton (Values L mem).BoundedStream (Values L mem).BoundedKeyedSingleton (outs) :=
  fun i s inp => @Prod.mk (BoundedOf (Values L mem).BoundedSingleton (Values L mem).BoundedStream (Values L mem).BoundedKeyedSingleton (sts))
    (BoundedOf (Values L mem).BoundedSingleton (Values L mem).BoundedStream (Values L mem).BoundedKeyedSingleton outs)
    (projRR sts (g i (embedRR sts s) (embedRR ins inp)).1)
    (projRR outs (g i (embedRR sts s) (embedRR ins inp)).2)
def gC (g : Fin (mem ℓ) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (CoupleSem L mem pacing Tc Td hjT).BoundedKeyedSingleton (sts) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (CoupleSem L mem pacing Tc Td hjT).BoundedKeyedSingleton (ins) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (CoupleSem L mem pacing Tc Td hjT).BoundedKeyedSingleton (sts) × BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (CoupleSem L mem pacing Tc Td hjT).BoundedKeyedSingleton (outs)) : Fin (mem ℓ) → BoundedOf (SchedSem L mem pacing).BoundedSingleton (SchedSem L mem pacing).BoundedStream (SchedSem L mem pacing).BoundedKeyedSingleton (sts) → BoundedOf (SchedSem L mem pacing).BoundedSingleton (SchedSem L mem pacing).BoundedStream (SchedSem L mem pacing).BoundedKeyedSingleton (ins) →
    BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (CoupleSem L mem pacing Tc Td hjT).BoundedKeyedSingleton (sts) × BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (CoupleSem L mem pacing Tc Td hjT).BoundedKeyedSingleton (outs) :=
  fun i s inp => g i (embed2 sts s (coerce sts s)) (embed2 ins inp (coerce ins inp))
def W1 (g : Fin (mem ℓ) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (CoupleSem L mem pacing Tc Td hjT).BoundedKeyedSingleton (sts) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (CoupleSem L mem pacing Tc Td hjT).BoundedKeyedSingleton (ins) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (CoupleSem L mem pacing Tc Td hjT).BoundedKeyedSingleton (sts) × BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (CoupleSem L mem pacing Tc Td hjT).BoundedKeyedSingleton (outs)) : Prop := ∀ i s inp,
  (gS sts ins outs g i s inp).1 = projSR sts (gC sts ins outs g i s inp).1
    ∧ (gS sts ins outs g i s inp).2 = projSR outs (gC sts ins outs g i s inp).2
def W2 (g : Fin (mem ℓ) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (CoupleSem L mem pacing Tc Td hjT).BoundedKeyedSingleton (sts) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (CoupleSem L mem pacing Tc Td hjT).BoundedKeyedSingleton (ins) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (CoupleSem L mem pacing Tc Td hjT).BoundedKeyedSingleton (sts) × BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (CoupleSem L mem pacing Tc Td hjT).BoundedKeyedSingleton (outs)) : Prop :=
  ∀ i (s : BoundedOf (SchedSem L mem pacing).BoundedSingleton (SchedSem L mem pacing).BoundedStream (SchedSem L mem pacing).BoundedKeyedSingleton (sts)) inp,
  (gR sts ins outs g i (coerce sts s) (coerce ins inp)).1 = projRR sts (gC sts ins outs g i s inp).1
    ∧ (gR sts ins outs g i (coerce sts s) (coerce ins inp)).2 = projRR outs (gC sts ins outs g i s inp).2
def W3 (g : Fin (mem ℓ) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (CoupleSem L mem pacing Tc Td hjT).BoundedKeyedSingleton (sts) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (CoupleSem L mem pacing Tc Td hjT).BoundedKeyedSingleton (ins) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (CoupleSem L mem pacing Tc Td hjT).BoundedKeyedSingleton (sts) × BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (CoupleSem L mem pacing Tc Td hjT).BoundedKeyedSingleton (outs)) : Prop := ∀ i s inp,
  WfB sts (gC sts ins outs g i s inp).1 ∧ WfB outs (gC sts ins outs g i s inp).2
def scanWf (x : AtomicOf (CoupleSem L mem pacing Tc Td hjT).Atomic (CoupleSem L mem pacing Tc Td hjT).AtomicStream (CoupleSem L mem pacing Tc Td hjT).AtomicKeyedSingleton ℓ (ins)) (g : Fin (mem ℓ) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (CoupleSem L mem pacing Tc Td hjT).BoundedKeyedSingleton (sts) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (CoupleSem L mem pacing Tc Td hjT).BoundedKeyedSingleton (ins) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (CoupleSem L mem pacing Tc Td hjT).BoundedKeyedSingleton (sts) × BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (CoupleSem L mem pacing Tc Td hjT).BoundedKeyedSingleton (outs)) : Prop :=
  Wf ins x ∧ CoTick.allEO sts = true ∧ CoTick.allEO ins = true ∧ CoTick.allEO outs = true
    ∧ W1 sts ins outs g ∧ W2 sts ins outs g ∧ W3 sts ins outs g

/-! The rules. -/

theorem srLegs_pair (x : AtomicOf (CoupleSem L mem pacing Tc Td hjT).Atomic (CoupleSem L mem pacing Tc Td hjT).AtomicStream (CoupleSem L mem pacing Tc Td hjT).AtomicKeyedSingleton ℓ (.pair a b)) :
    srLegs (.pair a b) x = (srLegs a x.1, srLegs b x.2) := rfl
theorem srLegs_sing (x : (CoupleSem L mem pacing Tc Td hjT).Atomic ℓ τ) : srLegs (.sing τ) x = x.sr := rfl
theorem srLegs_stream (x : (CoupleSem L mem pacing Tc Td hjT).AtomicStream ℓ α ord ret) : srLegs (.stream α inst ord ret) x = x.sr := rfl
theorem rrLegs_pair (x : AtomicOf (CoupleSem L mem pacing Tc Td hjT).Atomic (CoupleSem L mem pacing Tc Td hjT).AtomicStream (CoupleSem L mem pacing Tc Td hjT).AtomicKeyedSingleton ℓ (.pair a b)) :
    rrLegs (.pair a b) x = (rrLegs a x.1, rrLegs b x.2) := rfl
theorem rrLegs_sing (x : (CoupleSem L mem pacing Tc Td hjT).Atomic ℓ τ) : rrLegs (.sing τ) x = x.rr := rfl
theorem rrLegs_stream (x : (CoupleSem L mem pacing Tc Td hjT).AtomicStream ℓ α ord ret) : rrLegs (.stream α inst ord ret) x = x.rr := rfl
theorem Wf_pair (x : AtomicOf (CoupleSem L mem pacing Tc Td hjT).Atomic (CoupleSem L mem pacing Tc Td hjT).AtomicStream (CoupleSem L mem pacing Tc Td hjT).AtomicKeyedSingleton ℓ (.pair a b)) : Wf (.pair a b) x = (Wf a x.1 ∧ Wf b x.2) := rfl
theorem Wf_sing (x : (CoupleSem L mem pacing Tc Td hjT).Atomic ℓ τ) : Wf (.sing τ) x = x.wf := rfl
theorem Wf_stream (x : (CoupleSem L mem pacing Tc Td hjT).AtomicStream ℓ α ord ret) : Wf (.stream α inst ord ret) x = x.wf := rfl

theorem projSR_pair (x : BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (CoupleSem L mem pacing Tc Td hjT).BoundedKeyedSingleton (.pair a b)) :
    projSR (.pair a b) x = (projSR a x.1, projSR b x.2) := rfl
theorem projSR_sing (x : (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton τ) : projSR (.sing τ) x = x.sr := rfl
theorem projSR_stream (x : (CoupleSem L mem pacing Tc Td hjT).BoundedStream α ord ret) : projSR (.stream α inst ord ret) x = x.sr := rfl
theorem projRR_pair (x : BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (CoupleSem L mem pacing Tc Td hjT).BoundedKeyedSingleton (.pair a b)) :
    projRR (.pair a b) x = (projRR a x.1, projRR b x.2) := rfl
theorem projRR_sing (x : (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton τ) : projRR (.sing τ) x = x.rr := rfl
theorem projRR_stream (x : (CoupleSem L mem pacing Tc Td hjT).BoundedStream α ord ret) : projRR (.stream α inst ord ret) x = x.rr := rfl
theorem WfB_pair (x : BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (CoupleSem L mem pacing Tc Td hjT).BoundedKeyedSingleton (.pair a b)) : WfB (.pair a b) x = (WfB a x.1 ∧ WfB b x.2) := rfl
theorem WfB_sing (x : (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton τ) : WfB (.sing τ) x = x.wf := rfl
theorem WfB_stream (x : (CoupleSem L mem pacing Tc Td hjT).BoundedStream α ord ret) : WfB (.stream α inst ord ret) x = x.wf := rfl

theorem embedSR_pair (x : BoundedOf (SchedSem L mem pacing).BoundedSingleton (SchedSem L mem pacing).BoundedStream (SchedSem L mem pacing).BoundedKeyedSingleton (.pair a b)) :
    embedSR (Td := Td) (hjT := hjT) (.pair a b) x = (embedSR (Td := Td) (hjT := hjT) a x.1, embedSR (Td := Td) (hjT := hjT) b x.2) := rfl
theorem embedSR_sing_sr (v : (SchedSem L mem pacing).BoundedSingleton τ) : (embedSR (Td := Td) (hjT := hjT) (.sing τ) v).sr = v := rfl
theorem embedSR_sing_rr (v : (SchedSem L mem pacing).BoundedSingleton τ) : (embedSR (Td := Td) (hjT := hjT) (.sing τ) v).rr = idV v := rfl
theorem embedSR_sing_wf (v : (SchedSem L mem pacing).BoundedSingleton τ) : (embedSR (Td := Td) (hjT := hjT) (.sing τ) v).wf = False := rfl
theorem embedSR_stream_sr (l : (SchedSem L mem pacing).BoundedStream α ord ret) :
    (embedSR (Td := Td) (hjT := hjT) (.stream α inst ord ret) l).sr = l := rfl
theorem embedSR_stream_wf (l : (SchedSem L mem pacing).BoundedStream α ord ret) :
    (embedSR (Td := Td) (hjT := hjT) (.stream α inst ord ret) l).wf = False := rfl
theorem embedRR_pair (x : BoundedOf (Values L mem).BoundedSingleton (Values L mem).BoundedStream (Values L mem).BoundedKeyedSingleton (.pair a b)) :
    embedRR (pacing := pacing) (Td := Td) (hjT := hjT) (.pair a b) x = (embedRR (pacing := pacing) (Td := Td) (hjT := hjT) a x.1, embedRR (pacing := pacing) (Td := Td) (hjT := hjT) b x.2) := rfl
theorem embedRR_sing_sr (v : (Values L mem).BoundedSingleton τ) : (embedRR (pacing := pacing) (Td := Td) (hjT := hjT) (.sing τ) v).sr = v := rfl
theorem embedRR_sing_rr (v : (Values L mem).BoundedSingleton τ) : (embedRR (pacing := pacing) (Td := Td) (hjT := hjT) (.sing τ) v).rr = v := rfl
theorem embedRR_sing_wf (v : (Values L mem).BoundedSingleton τ) : (embedRR (pacing := pacing) (Td := Td) (hjT := hjT) (.sing τ) v).wf = False := rfl
theorem embedRR_stream_rr (p : (Values L mem).BoundedStream α ord ret) :
    (embedRR (pacing := pacing) (Td := Td) (hjT := hjT) (.stream α inst ord ret) p).rr = p := rfl
theorem embedRR_stream_wf (p : (Values L mem).BoundedStream α ord ret) :
    (embedRR (pacing := pacing) (Td := Td) (hjT := hjT) (.stream α inst ord ret) p).wf = False := rfl
theorem embed2_pair (m : BoundedOf (SchedSem L mem pacing).BoundedSingleton (SchedSem L mem pacing).BoundedStream (SchedSem L mem pacing).BoundedKeyedSingleton (.pair a b)) (v : BoundedOf (Values L mem).BoundedSingleton (Values L mem).BoundedStream (Values L mem).BoundedKeyedSingleton (.pair a b)) :
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
theorem coerce_pair (m : BoundedOf (SchedSem L mem pacing).BoundedSingleton (SchedSem L mem pacing).BoundedStream (SchedSem L mem pacing).BoundedKeyedSingleton (.pair a b)) :
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
/-- The `.ksing` leaf rules (facade-typed). -/
theorem srLegs_ksing (K V : Type) (iK : DecidableEq K) (iV : DecidableEq V)
    (x : (CoupleSem L mem pacing Tc Td hjT).AtomicKeyedSingleton ℓ K V) :
    srLegs (.ksing K V iK iV) x = x.sr := rfl
theorem rrLegs_ksing (K V : Type) (iK : DecidableEq K) (iV : DecidableEq V)
    (x : (CoupleSem L mem pacing Tc Td hjT).AtomicKeyedSingleton ℓ K V) :
    rrLegs (.ksing K V iK iV) x = x.rr := rfl
theorem Wf_ksing (K V : Type) (iK : DecidableEq K) (iV : DecidableEq V)
    (x : (CoupleSem L mem pacing Tc Td hjT).AtomicKeyedSingleton ℓ K V) :
    Wf (.ksing K V iK iV) x = x.wf := rfl
theorem projSR_ksing (K V : Type) (iK : DecidableEq K) (iV : DecidableEq V)
    (x : (CoupleSem L mem pacing Tc Td hjT).BoundedKeyedSingleton K V) :
    projSR (.ksing K V iK iV) x = x.sr := rfl
theorem projRR_ksing (K V : Type) (iK : DecidableEq K) (iV : DecidableEq V)
    (x : (CoupleSem L mem pacing Tc Td hjT).BoundedKeyedSingleton K V) :
    projRR (.ksing K V iK iV) x = x.rr := rfl
theorem WfB_ksing (K V : Type) (iK : DecidableEq K) (iV : DecidableEq V)
    (x : (CoupleSem L mem pacing Tc Td hjT).BoundedKeyedSingleton K V) :
    WfB (.ksing K V iK iV) x = x.wf := rfl
theorem embedSR_ksing_sr (K V : Type) (iK : DecidableEq K) (iV : DecidableEq V)
    (m : (SchedSem L mem pacing).BoundedKeyedSingleton K V) :
    (embedSR (Td := Td) (hjT := hjT) (.ksing K V iK iV) m).sr = m := rfl
theorem embedSR_ksing_wf (K V : Type) (iK : DecidableEq K) (iV : DecidableEq V)
    (m : (SchedSem L mem pacing).BoundedKeyedSingleton K V) :
    (embedSR (Td := Td) (hjT := hjT) (.ksing K V iK iV) m).wf = False := rfl
theorem embedRR_ksing_rr (K V : Type) (iK : DecidableEq K) (iV : DecidableEq V)
    (v : (Values L mem).BoundedKeyedSingleton K V) :
    (embedRR (pacing := pacing) (Td := Td) (hjT := hjT) (.ksing K V iK iV) v).rr = v := rfl
theorem embedRR_ksing_wf (K V : Type) (iK : DecidableEq K) (iV : DecidableEq V)
    (v : (Values L mem).BoundedKeyedSingleton K V) :
    (embedRR (pacing := pacing) (Td := Td) (hjT := hjT) (.ksing K V iK iV) v).wf = False := rfl
theorem embed2_ksing_sr (K V : Type) (iK : DecidableEq K) (iV : DecidableEq V)
    (m : (SchedSem L mem pacing).BoundedKeyedSingleton K V)
    (v : (Values L mem).BoundedKeyedSingleton K V) :
    (embed2 (Td := Td) (hjT := hjT) (.ksing K V iK iV) m v).sr = m := rfl
theorem embed2_ksing_rr (K V : Type) (iK : DecidableEq K) (iV : DecidableEq V)
    (m : (SchedSem L mem pacing).BoundedKeyedSingleton K V)
    (v : (Values L mem).BoundedKeyedSingleton K V) :
    (embed2 (Td := Td) (hjT := hjT) (.ksing K V iK iV) m v).rr = v := rfl
theorem embed2_ksing_wf (K V : Type) (iK : DecidableEq K) (iV : DecidableEq V)
    (m : (SchedSem L mem pacing).BoundedKeyedSingleton K V)
    (v : (Values L mem).BoundedKeyedSingleton K V) :
    (embed2 (Td := Td) (hjT := hjT) (.ksing K V iK iV) m v).wf = (KeyedMap.ofAssoc m = v) := rfl
theorem coerce_ksing (K V : Type) (iK : DecidableEq K) (iV : DecidableEq V)
    (m : (SchedSem L mem pacing).BoundedKeyedSingleton K V) :
    coerce (.ksing K V iK iV) m = KeyedMap.ofAssoc m := rfl
theorem eq_idV_self (m : (SchedSem L mem pacing).BoundedSingleton σ) : (m = idV m) = True :=
  propext ⟨fun _ => trivial, fun _ => rfl⟩
theorem gS_def (g : Fin (mem ℓ) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (CoupleSem L mem pacing Tc Td hjT).BoundedKeyedSingleton (sts) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (CoupleSem L mem pacing Tc Td hjT).BoundedKeyedSingleton (ins) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (CoupleSem L mem pacing Tc Td hjT).BoundedKeyedSingleton (sts) × BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (CoupleSem L mem pacing Tc Td hjT).BoundedKeyedSingleton (outs)) : gS sts ins outs g = fun i s inp =>
    @Prod.mk (BoundedOf (SchedSem L mem pacing).BoundedSingleton (SchedSem L mem pacing).BoundedStream (SchedSem L mem pacing).BoundedKeyedSingleton (sts))
      (BoundedOf (SchedSem L mem pacing).BoundedSingleton (SchedSem L mem pacing).BoundedStream (SchedSem L mem pacing).BoundedKeyedSingleton outs)
      (projSR sts (g i (embedSR sts s) (embedSR ins inp)).1)
      (projSR outs (g i (embedSR sts s) (embedSR ins inp)).2) := rfl
theorem gR_def (g : Fin (mem ℓ) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (CoupleSem L mem pacing Tc Td hjT).BoundedKeyedSingleton (sts) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (CoupleSem L mem pacing Tc Td hjT).BoundedKeyedSingleton (ins) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (CoupleSem L mem pacing Tc Td hjT).BoundedKeyedSingleton (sts) × BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (CoupleSem L mem pacing Tc Td hjT).BoundedKeyedSingleton (outs)) : gR sts ins outs g = fun i s inp =>
    @Prod.mk (BoundedOf (Values L mem).BoundedSingleton (Values L mem).BoundedStream (Values L mem).BoundedKeyedSingleton (sts))
      (BoundedOf (Values L mem).BoundedSingleton (Values L mem).BoundedStream (Values L mem).BoundedKeyedSingleton outs)
      (projRR sts (g i (embedRR sts s) (embedRR ins inp)).1)
      (projRR outs (g i (embedRR sts s) (embedRR ins inp)).2) := rfl
theorem gC_def (g : Fin (mem ℓ) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (CoupleSem L mem pacing Tc Td hjT).BoundedKeyedSingleton (sts) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (CoupleSem L mem pacing Tc Td hjT).BoundedKeyedSingleton (ins) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (CoupleSem L mem pacing Tc Td hjT).BoundedKeyedSingleton (sts) × BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (CoupleSem L mem pacing Tc Td hjT).BoundedKeyedSingleton (outs)) : gC sts ins outs g = fun i s inp =>
    g i (embed2 sts s (coerce sts s)) (embed2 ins inp (coerce ins inp)) := rfl
theorem W1_def (g : Fin (mem ℓ) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (CoupleSem L mem pacing Tc Td hjT).BoundedKeyedSingleton (sts) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (CoupleSem L mem pacing Tc Td hjT).BoundedKeyedSingleton (ins) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (CoupleSem L mem pacing Tc Td hjT).BoundedKeyedSingleton (sts) × BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (CoupleSem L mem pacing Tc Td hjT).BoundedKeyedSingleton (outs)) : W1 sts ins outs g = ∀ i s inp,
    (gS sts ins outs g i s inp).1 = projSR sts (gC sts ins outs g i s inp).1
      ∧ (gS sts ins outs g i s inp).2 = projSR outs (gC sts ins outs g i s inp).2 := rfl
theorem W2_def (g : Fin (mem ℓ) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (CoupleSem L mem pacing Tc Td hjT).BoundedKeyedSingleton (sts) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (CoupleSem L mem pacing Tc Td hjT).BoundedKeyedSingleton (ins) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (CoupleSem L mem pacing Tc Td hjT).BoundedKeyedSingleton (sts) × BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (CoupleSem L mem pacing Tc Td hjT).BoundedKeyedSingleton (outs)) : W2 sts ins outs g = ∀ i (s : BoundedOf (SchedSem L mem pacing).BoundedSingleton (SchedSem L mem pacing).BoundedStream (SchedSem L mem pacing).BoundedKeyedSingleton (sts)) inp,
    (gR sts ins outs g i (coerce sts s) (coerce ins inp)).1 = projRR sts (gC sts ins outs g i s inp).1
      ∧ (gR sts ins outs g i (coerce sts s) (coerce ins inp)).2 = projRR outs (gC sts ins outs g i s inp).2 := rfl
theorem W3_def (g : Fin (mem ℓ) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (CoupleSem L mem pacing Tc Td hjT).BoundedKeyedSingleton (sts) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (CoupleSem L mem pacing Tc Td hjT).BoundedKeyedSingleton (ins) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (CoupleSem L mem pacing Tc Td hjT).BoundedKeyedSingleton (sts) × BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (CoupleSem L mem pacing Tc Td hjT).BoundedKeyedSingleton (outs)) : W3 sts ins outs g = ∀ i s inp,
    WfB sts (gC sts ins outs g i s inp).1 ∧ WfB outs (gC sts ins outs g i s inp).2 := rfl
theorem scanWf_def (x : AtomicOf (CoupleSem L mem pacing Tc Td hjT).Atomic (CoupleSem L mem pacing Tc Td hjT).AtomicStream (CoupleSem L mem pacing Tc Td hjT).AtomicKeyedSingleton ℓ (ins)) (g : Fin (mem ℓ) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (CoupleSem L mem pacing Tc Td hjT).BoundedKeyedSingleton (sts) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (CoupleSem L mem pacing Tc Td hjT).BoundedKeyedSingleton (ins) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (CoupleSem L mem pacing Tc Td hjT).BoundedKeyedSingleton (sts) × BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (CoupleSem L mem pacing Tc Td hjT).BoundedKeyedSingleton (outs)) :
    scanWf sts ins outs x g
      = (Wf ins x ∧ CoTick.allEO sts = true ∧ CoTick.allEO ins = true ∧ CoTick.allEO outs = true
        ∧ W1 sts ins outs g ∧ W2 sts ins outs g ∧ W3 sts ins outs g) := rfl

/-- The coupling, at the facade's types (the proof argument of the
corner's `mk`; its TYPE must be spelled through the facade too, or the
`mk` projection rules cannot unify it). -/
theorem scan_cpl (x : AtomicOf (CoupleSem L mem pacing Tc Td hjT).Atomic (CoupleSem L mem pacing Tc Td hjT).AtomicStream (CoupleSem L mem pacing Tc Td hjT).AtomicKeyedSingleton ℓ (ins)) (g : Fin (mem ℓ) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (CoupleSem L mem pacing Tc Td hjT).BoundedKeyedSingleton (sts) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (CoupleSem L mem pacing Tc Td hjT).BoundedKeyedSingleton (ins) → BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (CoupleSem L mem pacing Tc Td hjT).BoundedKeyedSingleton (sts) × BoundedOf (CoupleSem L mem pacing Tc Td hjT).BoundedSingleton (CoupleSem L mem pacing Tc Td hjT).BoundedStream (CoupleSem L mem pacing Tc Td hjT).BoundedKeyedSingleton (outs)) (init : SeedOf sts) :
    scanWf sts ins outs x g → ∀ k, k ≤ Tc →
      CorrTick.Le outs ((SchedSem L mem pacing).tick_scan sts ins outs (srLegs ins x) (gS sts ins outs g) init) k
        ((Values L mem).tick_scan sts ins outs (rrLegs ins x) (gR sts ins outs g) init) :=
  CoTick.scan_cpl pacing sts ins outs x g init

end CoTickI

/-! ### The shaped tick former at the corner: leg projections by output
PATH, structurally. The corner's `mk` is shape-recursive, so for ANY
projection path `π` into the output tuple and any leaf kind,
`(π (tick_scan sts ins outs x g init)).sr` is definitionally
`π (SchedSem.tick_scan sts ins outs (srLegs ins x) (gS sts ins outs g) init)`
(`.rr` to the `Values` former on the `rrLegs`, `.wf` to `scanWf`). One
simproc computes the path and emits that right-hand side — a
proof-free (definitional) step — in place of the former one-rule-per-
(path, leaf kind, leg) table (D70: the table was hardcoded to right-
nested outputs of four legs and knew no `.ksing` leaf). It is keyed on
the leg projection of a projected former, so it fires only where the
walk's `.sr`/`.rr`/`.wf` reaches a former's output — never on a bare
`tick_scan` inside a callee's argument — and its right-hand side has
no proof argument (the `mk`-with-coupling spelling has one, whose type
the walk cannot keep in step with the rewritten legs). -/
section TickScanProj
open Lean Meta Simp

/-- A `Prod.fst`/`Prod.snd` chain down to its root; the path is listed
innermost first (`true` = `.1`). -/
partial def stripProdProj (e : Expr) : Expr × List Bool :=
  if e.isAppOfArity ``Prod.fst 3 then
    let (r, p) := stripProdProj (e.getArg! 2); (r, p ++ [true])
  else if e.isAppOfArity ``Prod.snd 3 then
    let (r, p) := stripProdProj (e.getArg! 2); (r, p ++ [false])
  else (e.consumeMData, [])

/-- Re-apply a projection path. -/
def applyProdPath (root : Expr) (path : List Bool) : MetaM Expr :=
  path.foldlM (fun acc isFst =>
    mkAppM (if isFst then ``Prod.fst else ``Prod.snd) #[acc]) root

/-- The leg a corner leaf projection names: `sr` ↦ 0, `rr` ↦ 1, `wf` ↦ 2. -/
def coLegOfProj : Name → Option Nat
  | ``CoTickSing.sr | ``CoAtomicStream.sr | ``CoAtomicKSing.sr => some 0
  | ``CoTickSing.rr | ``CoAtomicStream.rr | ``CoAtomicKSing.rr => some 1
  | ``CoTickSing.wf | ``CoAtomicStream.wf | ``CoAtomicKSing.wf => some 2
  | _ => none

/-- `coTickScanProj`: the leg projection of a projected corner `tick_scan`
(see the section header). -/
simproc_decl coTickScanProj (_) := fun e => do
  let e := e.consumeMData
  -- the leaf projection (as a projection-function application or a raw
  -- `.proj` node) and its subject
  let r? : Option (Nat × Expr) :=
    match e with
    | .proj sn idx s =>
      if (sn == ``CoTickSing || sn == ``CoAtomicStream || sn == ``CoAtomicKSing) && idx ≤ 2
      then some (idx, s) else none
    | _ =>
      match e.getAppFn with
      | .const pn _ => if e.isApp then (coLegOfProj pn).map fun leg => (leg, e.appArg!) else none
      | _ => none
  let some (leg, subj) := r? | return .continue
  let (root, path) := stripProdProj subj
  unless root.isAppOfArity ``HydroSem.tick_scan 10 do return .continue
  let args := root.getAppArgs
  let H := args[2]!.consumeMData
  unless H.isAppOfArity ``CoupleSem 6 do return .continue
  let hargs := H.getAppArgs
  let (L, mem, pacing) := (hargs[0]!, hargs[1]!, hargs[2]!)
  let (ℓ, sts, ins, outs, x, g, init) :=
    (args[3]!, args[4]!, args[5]!, args[6]!, args[7]!, args[8]!, args[9]!)
  -- the former at another interpretation: the same head constant (with
  -- its universe levels) over the other interpretation's legs
  let formerAt (interp legs body : Expr) : Expr :=
    mkAppN root.getAppFn #[L, mem, interp, ℓ, sts, ins, outs, legs, body, init]
  -- the `CoTickI` helpers, their implicit prefix `{L mem pacing Tc Td
  -- hjT ℓ}` spelled from the corner's arguments (inference from the
  -- body's type is not always possible: its binder types may be the
  -- raw corner carriers)
  let helper (n : Name) (rest : Array Expr) : Expr :=
    mkAppN (mkConst n) (hargs ++ #[ℓ] ++ rest)
  let rhs ← match leg with
    | 0 =>
      let sched := mkAppN (mkConst ``SchedSem) #[L, mem, pacing]
      applyProdPath (formerAt sched (helper ``CoTickI.srLegs #[ins, x])
        (helper ``CoTickI.gS #[sts, ins, outs, g])) path
    | 1 =>
      let vals := mkAppN (mkConst ``Values) #[L, mem]
      applyProdPath (formerAt vals (helper ``CoTickI.rrLegs #[ins, x])
        (helper ``CoTickI.gR #[sts, ins, outs, g])) path
    | _ => pure (helper ``CoTickI.scanWf #[sts, ins, outs, x, g])
  return .visit { expr := rhs }

end TickScanProj

end InTick

theorem co_defer_tick_rr {σ : Type} (init : σ)
    (t : (CoupleSem L mem pacing Tc Td hjT).Atomic ℓ σ)
    :
    ((CoupleSem L mem pacing Tc Td hjT).defer_tick (ℓ := ℓ) init t).rr
      = (Values L mem).defer_tick (ℓ := ℓ) init t.rr := rfl

theorem co_defer_tick_sr {σ : Type} (init : σ)
    (t : (CoupleSem L mem pacing Tc Td hjT).Atomic ℓ σ) :
    ((CoupleSem L mem pacing Tc Td hjT).defer_tick (ℓ := ℓ) init t).sr
      = (SchedSem L mem pacing).defer_tick (ℓ := ℓ) init t.sr := rfl

theorem co_defer_tick_wf {σ : Type} (init : σ)
    (t : (CoupleSem L mem pacing Tc Td hjT).Atomic ℓ σ) :
    ((CoupleSem L mem pacing Tc Td hjT).defer_tick (ℓ := ℓ) init t).wf
      = t.wf := rfl

theorem co_flattenOrdered_rr {β : Type} [DecidableEq β]
    (t : (CoupleSem L mem pacing Tc Td hjT).Atomic ℓ (List β))
    :
    ((CoupleSem L mem pacing Tc Td hjT).flattenOrdered (ℓ := ℓ) t).rr
      = (Values L mem).flattenOrdered (ℓ := ℓ) t.rr := rfl

theorem co_flattenOrdered_sr {β : Type} [DecidableEq β]
    (t : (CoupleSem L mem pacing Tc Td hjT).Atomic ℓ (List β)) :
    ((CoupleSem L mem pacing Tc Td hjT).flattenOrdered (ℓ := ℓ) t).sr
      = (SchedSem L mem pacing).flattenOrdered (ℓ := ℓ) t.sr := rfl

theorem co_flattenOrdered_wf {β : Type} [DecidableEq β]
    (t : (CoupleSem L mem pacing Tc Td hjT).Atomic ℓ (List β)) :
    ((CoupleSem L mem pacing Tc Td hjT).flattenOrdered (ℓ := ℓ) t).wf
      = t.wf := rfl

theorem co_flattenUnordered_rr {β : Type} [DecidableEq β]
    (t : (CoupleSem L mem pacing Tc Td hjT).Atomic ℓ (List β))
    :
    ((CoupleSem L mem pacing Tc Td hjT).flattenUnordered (ℓ := ℓ) t).rr
      = (Values L mem).flattenUnordered (ℓ := ℓ) t.rr := rfl

theorem co_flattenUnordered_sr {β : Type} [DecidableEq β]
    (t : (CoupleSem L mem pacing Tc Td hjT).Atomic ℓ (List β)) :
    ((CoupleSem L mem pacing Tc Td hjT).flattenUnordered (ℓ := ℓ) t).sr
      = (SchedSem L mem pacing).flattenUnordered (ℓ := ℓ) t.sr
      := rfl

theorem co_flattenUnordered_wf {β : Type} [DecidableEq β]
    (t : (CoupleSem L mem pacing Tc Td hjT).Atomic ℓ (List β)) :
    ((CoupleSem L mem pacing Tc Td hjT).flattenUnordered (ℓ := ℓ) t).wf
      = t.wf := rfl

theorem co_allTicks_rr {β : Type} [DecidableEq β]
    {ord : StrOrd}
    (bs : (CoupleSem L mem pacing Tc Td hjT).AtomicStream ℓ β ord .exactlyOnce)
    :
    ((CoupleSem L mem pacing Tc Td hjT).allTicks (ℓ := ℓ) (ord := ord) bs).rr
      = (Values L mem).allTicks (ℓ := ℓ) (ord := ord) bs.rr := rfl

theorem co_allTicks_sr {β : Type} [DecidableEq β]
    {ord : StrOrd}
    (bs : (CoupleSem L mem pacing Tc Td hjT).AtomicStream ℓ β ord .exactlyOnce) :
    ((CoupleSem L mem pacing Tc Td hjT).allTicks (ℓ := ℓ) (ord := ord) bs).sr
      = (SchedSem L mem pacing).allTicks (ℓ := ℓ) (ord := ord) bs.sr
      := rfl

theorem co_allTicks_wf {β : Type} [DecidableEq β]
    {ord : StrOrd}
    (bs : (CoupleSem L mem pacing Tc Td hjT).AtomicStream ℓ β ord .exactlyOnce) :
    ((CoupleSem L mem pacing Tc Td hjT).allTicks (ℓ := ℓ) (ord := ord) bs).wf
      = bs.wf := rfl

theorem co_sample_every_rr {α : Type} [DecidableEq α]
    (t : (CoupleSem L mem pacing Tc Td hjT).Atomic ℓ (Option α))
    (times : (CoupleSem L mem pacing Tc Td hjT).SampleDec (mem ℓ)) :
    ((CoupleSem L mem pacing Tc Td hjT).sample_every t times).rr
      = (Values L mem).sample_every (ℓ := ℓ) t.rr times := rfl

theorem co_sample_every_sr {α : Type} [DecidableEq α]
    (t : (CoupleSem L mem pacing Tc Td hjT).Atomic ℓ (Option α))
    (times : (CoupleSem L mem pacing Tc Td hjT).SampleDec (mem ℓ)) :
    ((CoupleSem L mem pacing Tc Td hjT).sample_every t times).sr
      = (SchedSem L mem pacing).sample_every (ℓ := ℓ) t.sr times := rfl

theorem co_sample_every_wf {α : Type} [DecidableEq α]
    (t : (CoupleSem L mem pacing Tc Td hjT).Atomic ℓ (Option α))
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
    (body : ∀ H', Γ H' → H'.Atomic ℓ σ →
      H'.Atomic ℓ σ) :
    (SchedSem L mem pacing).Atomic ℓ σ :=
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
    (body : ∀ H', Γ H' → H'.Atomic ℓ σ →
      H'.Atomic ℓ σ) :
    ((CoupleSem L mem pacing Tc Td hjT).fixTick (ℓ := ℓ) df caps
        body).rr
      = (Values L mem).fix_tick (ℓ := ℓ) (σ := σ) (Td + 1)
          (fun v => (body _ caps (CoTickSing.probeC
            (CoTickSing.fixSrC caps body) v)).rr) := rfl

theorem co_hfix_tick_sr {σ : Type}
    (df : (CoupleSem L mem pacing Tc Td hjT).FixDec)
    (caps : Γ (CoupleSem L mem pacing Tc Td hjT))
    (body : ∀ H', Γ H' → H'.Atomic ℓ σ →
      H'.Atomic ℓ σ) :
    ((CoupleSem L mem pacing Tc Td hjT).fixTick (ℓ := ℓ) df caps
        body).sr
      = CoTickSing.fixSrC caps body := rfl

theorem co_tick_fixSrC_eq {σ : Type}
    (caps : Γ (CoupleSem L mem pacing Tc Td hjT))
    (body : ∀ H', Γ H' → H'.Atomic ℓ σ →
      H'.Atomic ℓ σ) :
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
    (body : ∀ H', Γ H' → H'.Atomic ℓ σ →
      H'.Atomic ℓ σ) :
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
    (SchedSem L mem pacing).snapshot (ℓ := ℓ) (α := α) (ord := ord) (ret := ret)
        (b := b) s u
      = (SchedSem L mem pacing).snapshot (ℓ := ℓ) (α := α) (ord := ord)
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
    (body : (SchedSem L mem pacing).Atomic ℓ σ →
      (SchedSem L mem pacing).Atomic ℓ σ) :
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
    (body : ∀ H', Γ H' → H'.Atomic ℓ σ →
      H'.Atomic ℓ σ) :
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
  ``co_input_rr, ``co_input_sr, ``co_sing_input_rr, ``co_sing_input_sr,
  ``co_sing_input_src, ``co_tick_input_rr,
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
  ``co_fold_sr, ``co_fold_src, ``co_fold_monotone_rr, ``co_fold_monotone_sr,
  ``co_fold_monotone_src,
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
  ``co_binto_keyed_rr, ``co_binto_keyed_sr, ``co_bkfold_rr, ``co_bkfold_sr,
  ``co_bksFilter_rr, ``co_bksFilter_sr, ``co_bksMap_rr, ``co_bksMap_sr,
  ``co_bksKeys_rr, ``co_bksKeys_sr, ``co_bksEntries_rr, ``co_bksEntries_sr,
  ``co_into_keyed_rr, ``co_into_keyed_sr, ``co_kassume_ordering_rr, ``co_kassume_ordering_sr,
  ``co_kfold_early_stop_rr, ``co_kfold_early_stop_sr, ``co_kfold_early_stop_src,
  ``co_kget_max_key_rr, ``co_kget_max_key_sr, ``co_kget_max_key_src,
  ``co_tinto_keyed_rr, ``co_tinto_keyed_sr, ``co_tassume_ordering_rr, ``co_tassume_ordering_sr,
  ``co_reduce_watermark_rr, ``co_reduce_watermark_sr,
  ``co_bjoin_rr, ``co_bjoin_sr,
  ``co_bantiJoin_rr, ``co_bantiJoin_sr, ``co_bfilterNotIn_rr, ``co_bmax_rr, ``co_bmax_sr,
  ``co_bfilterNotIn_sr, ``co_bfilterIf_rr, ``co_bfilterIf_sr,
  ``co_ite_bstream_sr, ``co_ite_bstream_rr, ``co_ite_bsing_sr,
  ``co_ite_bsing_rr, ``fst_ite, ``snd_ite,
  ``coTickScanProj,
  ``CoTickI.srLegs_pair, ``CoTickI.srLegs_sing, ``CoTickI.srLegs_stream, ``CoTickI.srLegs_ksing,
  ``CoTickI.rrLegs_pair, ``CoTickI.rrLegs_sing, ``CoTickI.rrLegs_stream, ``CoTickI.rrLegs_ksing,
  ``CoTickI.gS_def, ``CoTickI.gR_def, ``CoTickI.gC_def,
  ``CoTickI.projSR_pair, ``CoTickI.projSR_sing, ``CoTickI.projSR_stream, ``CoTickI.projSR_ksing,
  ``CoTickI.projRR_pair, ``CoTickI.projRR_sing, ``CoTickI.projRR_stream, ``CoTickI.projRR_ksing,
  ``CoTickI.embedSR_pair, ``CoTickI.embedSR_sing_sr, ``CoTickI.embedSR_sing_rr,
  ``CoTickI.embedSR_stream_sr, ``CoTickI.embedSR_ksing_sr, ``CoTickI.embedRR_pair, ``CoTickI.embedRR_sing_sr,
  ``CoTickI.embedRR_sing_rr, ``CoTickI.embedRR_stream_rr, ``CoTickI.embedRR_ksing_rr, ``CoTickI.embed2_pair,
  ``CoTickI.embed2_sing_sr, ``CoTickI.embed2_sing_rr, ``CoTickI.embed2_stream_sr,
  ``CoTickI.embed2_stream_rr, ``CoTickI.embed2_ksing_sr, ``CoTickI.embed2_ksing_rr,
  ``CoTickI.coerce_pair, ``CoTickI.coerce_sing, ``CoTickI.coerce_ksing,
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
  -- member reads of constructed `Values` carriers (D73: `MemberMap`)
  ``MemberMap.get_mk', ``MemberMap.get_mk,
  ``BoundedOf, ``AtomicOf]

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
  ``MemberMap.get_mk', ``MemberMap.get_mk,
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
  ``co_bweakenOrder_wf, ``co_bfilter_wf, ``co_binto_keyed_wf, ``co_bkfold_wf,
  ``co_bksFilter_wf, ``co_bksMap_wf, ``co_bksKeys_wf, ``co_bksEntries_wf,
  ``co_into_keyed_wf, ``co_kassume_ordering_wf, ``co_kfold_early_stop_wf,
  ``co_kget_max_key_wf, ``co_tinto_keyed_wf, ``co_tassume_ordering_wf, ``co_reduce_watermark_wf,
  ``co_bksEntries_rr_pairwise_key,
  ``co_bjoin_wf, ``co_bantiJoin_wf,
  ``co_bfilterNotIn_wf, ``co_bmax_wf, ``co_ite_bstream_sr, ``co_ite_bstream_rr,
  ``co_ite_bstream_wf, ``co_ite_bsing_sr, ``co_ite_bsing_rr,
  ``co_ite_bsing_wf, ``fst_ite, ``snd_ite,
  ``co_bfilterIf_wf, ``coTickScanProj, ``CoTickI.scanWf_def, ``CoTickI.Wf_pair,
  ``CoTickI.Wf_sing, ``CoTickI.Wf_stream, ``CoTickI.Wf_ksing, ``CoTick.allEO_sing,
  ``CoTick.allEO_stream_eo, ``CoTick.allEO_ksing, ``CoTick.allEO_pair_eq, ``Bool.and_self,
  ``Bool.true_and, ``Bool.and_true, ``CoTickI.W1_def,
  ``CoTickI.W2_def, ``CoTickI.W3_def, ``CoTickI.gS_def,
  ``CoTickI.gR_def, ``CoTickI.gC_def, ``CoTickI.projSR_pair,
  ``CoTickI.projSR_sing, ``CoTickI.projSR_stream, ``CoTickI.projSR_ksing, ``CoTickI.projRR_pair,
  ``CoTickI.projRR_sing, ``CoTickI.projRR_stream, ``CoTickI.projRR_ksing, ``CoTickI.WfB_pair,
  ``CoTickI.WfB_sing, ``CoTickI.WfB_stream, ``CoTickI.WfB_ksing, ``CoTickI.embedSR_pair,
  ``CoTickI.embedSR_sing_sr, ``CoTickI.embedSR_sing_rr, ``CoTickI.embedSR_sing_wf,
  ``CoTickI.embedSR_stream_sr, ``CoTickI.embedSR_stream_wf,
  ``CoTickI.embedSR_ksing_sr, ``CoTickI.embedSR_ksing_wf, ``CoTickI.embedRR_pair,
  ``CoTickI.embedRR_sing_sr, ``CoTickI.embedRR_sing_rr, ``CoTickI.embedRR_sing_wf,
  ``CoTickI.embedRR_stream_rr, ``CoTickI.embedRR_stream_wf,
  ``CoTickI.embedRR_ksing_rr, ``CoTickI.embedRR_ksing_wf, ``CoTickI.embed2_pair,
  ``CoTickI.embed2_sing_sr, ``CoTickI.embed2_sing_rr, ``CoTickI.embed2_sing_wf,
  ``CoTickI.embed2_stream_sr, ``CoTickI.embed2_stream_rr, ``CoTickI.embed2_stream_coerce_wf,
  ``CoTickI.embed2_ksing_sr, ``CoTickI.embed2_ksing_rr, ``CoTickI.embed2_ksing_wf,
  ``CoTickI.eq_idV_self, ``CoTickI.coerce_pair, ``CoTickI.coerce_sing, ``CoTickI.coerce_ksing,
  ``eq_self_iff_true, ``true_and, ``and_true,
  ``and_self, ``implies_true, ``ite_self,
  -- the conditional folds' pairwise conjunct at the unconditional
  -- obligation (`Pre := fun _ _ => True`) is `True`
  ``PoolPairwise_true, ``KeyedPool.Pairwise_true,
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
