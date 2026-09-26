import HydroV2.HydroRelLaws
import HydroV2.EagerProj

/-!
# HydroV2 · the eager-agreement relation (`HRel` instance)

The `Values` ↔ `Eager` correspondence as data: the relation is
denotation-projection equality (`e.den = v`, decisions shared —
`Eager`'s vocabulary *is* `Values`'s), and the op laws are the
`EagerProj` projection identities — op-scale `rfl`s — plus one
`congrArg` each for the two knot formers. `HydroParam.lean`'s
per-module free theorems instantiated here ARE the eager naming
machinery (the retired `EagerKnots.lean`'s content as corollaries
— see `Paxos/EagerCheck.lean`).

Unindexed guarantee: `I := Unit`.
-/

namespace HydroV2

/-- `.den` of a graded singleton cell pack, uniformly over the
bound. -/
def ESing.den {n : Nat} {α σ : Type} [DecidableEq α] {ord : StrOrd} :
    ∀ {b : SingBound σ}, ESing n α σ ord b → SingletonV n α σ ord b
  | .unbounded, e => EPack.den e
  | .monotonic _, e => EPack.den e

/-- `.den` of a graded tick pack, uniformly over the bound. -/
def ETick.den {n : Nat} {σ : Type} :
    ∀ {b : SingBound σ}, ETick n σ b → TickV n σ b
  | .unbounded, e => EPack.den e
  | .monotonic _, e => EPack.den e

/-- The eager-agreement relation families: denotation equality on
carriers, equality on (shared-vocabulary) decisions. -/
def eagC (L : Type) (mem : L → Nat) :
    HRelC Unit (Eager L mem) (Values L mem) where
  streamRel _ e v := e.den = v
  keyedRel _ e v := e.den = v
  singRel _ e v := e.den = v
  tickSingRel _ e v := e.den = v
  tickStreamRel _ e v := e.den = v
  transportRel _ d d' := d = d'
  orderSelRel _ d d' := d = d'
  snapRel _ d d' := d = d'
  batchRel _ d d' := d = d'
  ordBatchRel _ d d' := d = d'
  batchOrdSelRel _ d d' := d = d'
  sampleRel _ d d' := d = d'
  timerRel _ d d' := d = d'
  pulseRel _ d d' := d = d'
  emitRel _ d d' := d = d'
  fixRel _ d d' := d = d'

variable {L : Type} {mem : L → Nat}

/-- The uniform op-law discharge: expose the relations, substitute the
den equalities, close by the op-scale projection `rfl` (grade-matched
ops case on their bound first). -/
macro "eag_law_tac" : tactic =>
  `(tactic| (
    intro i
    intros
    try casesm* SingBound _
    all_goals simp only [eagC, ESing.den, ETick.den] at *
    all_goals subst_eqs
    all_goals rfl))

/-- The knot former laws: the eager `fix` den leg is the `Values` fix
of the embedded body, so pointwise body agreement transports by
`congrArg`. -/
theorem eag_law_fix_stream : HRel.law_fix_stream (eagC L mem) := by
  intro i ℓ α _ ord ret d d' hd b₁ b₂ hb
  have hd' : d = d' := hd
  subst hd'
  have hb' : ∀ v, (b₁ (EagStream.embedV v)).den = b₂ v :=
    fun v => hb i le_rfl (EagStream.embedV v) v (fun _ _ => rfl)
  show ((Eager L mem).fix_stream d b₁).den = (Values L mem).fix_stream d b₂
  exact (eag_fix_stream_den d b₁).trans
    (congrArg ((Values L mem).fix_stream d) (funext hb'))

theorem eag_law_fix_tick : HRel.law_fix_tick (eagC L mem) := by
  intro i ℓ σ d d' hd b₁ b₂ hb
  have hd' : d = d' := hd
  subst hd'
  have hb' : ∀ v, (b₁ (EagTickSing.embedV v)).den = b₂ v :=
    fun v => hb i le_rfl (EagTickSing.embedV v) v (fun _ _ => rfl)
  show ETick.den ((Eager L mem).fix_tick d b₁)
    = (Values L mem).fix_tick d b₂
  exact (eag_fix_tick_den d b₁).trans
    (congrArg ((Values L mem).fix_tick d) (funext hb'))

/-- The eager-agreement laws — one obligation per signature op, in
declaration order. -/
theorem eagLaws : HRel.Laws (eagC L mem) :=
  ⟨by eag_law_tac, -- map
   by eag_law_tac, -- filterMap
   by eag_law_tac, -- broadcast
   by eag_law_tac, -- demux
   by eag_law_tac, -- values
   by eag_law_tac, -- weaken_retries
   by eag_law_tac, -- union
   by eag_law_tac, -- assume_ordering
   by eag_law_tac, -- fold
   by eag_law_tac, -- fold_monotone
   by eag_law_tac, -- snapshot
   by eag_law_tac, -- batch
   by eag_law_tac, -- batch_ordered
   by eag_law_tac, -- assume_ordering_batch
   by eag_law_tac, -- mapBatchWith
   by eag_law_tac, -- mapBatch
   by eag_law_tac, -- mapBatchesWith
   by eag_law_tac, -- scan_batches_across_ticks
   by eag_law_tac, -- fold_batches_across_ticks_monotone
   by eag_law_tac, -- sample_every
   by eag_law_tac, -- timeout_snapshot
   by eag_law_tac, -- source_interval_batch
   by eag_law_tac, -- scan_batches_unordered_across_ticks
   by eag_law_tac, -- scan_batches_unordered
   by eag_law_tac, -- scan_batches_unordered₂
   by eag_law_tac, -- scan_across_ticks
   by eag_law_tac, -- mapTick
   by eag_law_tac, -- zipTick
   by eag_law_tac, -- fold_across_ticks_monotone
   by eag_law_tac, -- mapMonotone
   by eag_law_tac, -- forgetBound
   by eag_law_tac, -- defer
   by eag_law_tac, -- filterMapBatchesWith
   by eag_law_tac, -- mapBatchesUnordered
   by eag_law_tac, -- emitMultisetBatches
   by eag_law_tac, -- allTicks
   by eag_law_tac, -- emitBatches
   by eag_law_tac, -- emitBatchesUnordered
   eag_law_fix_stream,
   eag_law_fix_tick⟩

end HydroV2
