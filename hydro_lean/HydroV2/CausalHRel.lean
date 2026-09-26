import HydroV2.HydroRelLaws
import HydroV2.SchedCausal

/-!
# HydroV2 · the step-causality relation (`HRel` instance)

Machine-op causality — output views below a step horizon depend only
on input views below it — as an `HRel` instance: the relation is
agreement-below-`h` at the `SchedSem` diagonal (`SAgree`/`KAgree`/
`FAgree`/`TAgree`, index = the horizon), and the op laws are the
`SchedCausal.lean` congruences. The knot laws spend the strong-form
body premise through the relation's own downward closure
(`SAgree.mono`/`TAgree.mono`) — private to the instance, never
assumed by the signature. `HydroParam.lean`'s per-module free
theorems instantiated here ARE per-module causality — no per-module
generation, no walker.

Step-indexed guarantee: `I := Nat` (the horizon).
-/

namespace HydroV2

/-- The step-causality relation families: agreement below the horizon
at the machine diagonal (decisions equal). -/
def causalC (L : Type) (mem : L → Nat)
    (pacing : (ℓ : L) → Fin (mem ℓ) → Nat → Bool) :
    HRelC Nat (SchedSem L mem pacing) (SchedSem L mem pacing) where
  streamRel h x y := SAgree h x y
  keyedRel h x y := KAgree h x y
  singRel h x y := FAgree h x y
  tickSingRel h x y := TAgree h x y
  tickStreamRel h x y := TAgree h x y
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
  {pacing : (ℓ : L) → Fin (mem ℓ) → Nat → Bool}

/-- The uniform op-law discharge: expose the agreement relations,
substitute the decision equalities, close by the matching `causal_*`
congruence. -/
macro "causal_law_tac" : tactic =>
  `(tactic| (
    intro h
    intros
    try casesm* SingBound _
    all_goals simp only [causalC] at *
    all_goals subst_eqs
    all_goals solve
      | (apply causal_map <;> assumption)
      | (apply causal_filterMap <;> assumption)
      | (apply causal_broadcast <;> assumption)
      | (apply causal_demux <;> assumption)
      | (apply causal_values <;> assumption)
      | (apply causal_weaken_retries <;> assumption)
      | (apply causal_union <;> assumption)
      | (apply causal_assume_ordering <;> assumption)
      | (apply causal_fold <;> assumption)
      | (apply causal_fold_monotone <;> assumption)
      | (apply causal_snapshot <;> assumption)
      | (apply causal_batch <;> assumption)
      | (apply causal_batch_ordered <;> assumption)
      | (apply causal_assume_ordering_batch <;> assumption)
      | (apply causal_mapBatchWith <;> assumption)
      | (apply causal_mapBatch <;> assumption)
      | (apply causal_mapBatchesWith <;> assumption)
      | (apply causal_filterMapBatchesWith <;> assumption)
      | (apply causal_scan_batches_across_ticks <;> assumption)
      | (apply causal_fold_batches_across_ticks_monotone <;> assumption)
      | (apply causal_scan_batches_unordered_across_ticks <;> assumption)
      | (apply causal_scan_batches_unordered <;> assumption)
      | (apply causal_scan_batches_unordered₂ <;> assumption)
      | (apply causal_scan_across_ticks <;> assumption)
      | (apply causal_sample_every <;> assumption)
      | (apply causal_timeout_snapshot <;> assumption)
      | (apply causal_source_interval_batch <;> assumption)
      | (apply causal_mapTick <;> assumption)
      | (apply causal_zipTick <;> assumption)
      | (apply causal_fold_across_ticks_monotone <;> assumption)
      | (apply causal_mapMonotone <;> assumption)
      | (apply causal_forgetBound <;> assumption)
      | (apply causal_defer <;> assumption)
      | (apply causal_allTicks <;> assumption)
      | (apply causal_mapBatchesUnordered <;> assumption)
      | (apply causal_emitBatches <;> assumption)
      | (apply causal_emitMultisetBatches <;> assumption)
      | (apply causal_emitBatchesUnordered <;> assumption)))

/-- The knot former laws: the index-lowered strong-form premise is
exactly `causal_fix_*`'s shape, with the strong form spent through
downward closure. -/
theorem causal_law_fix_stream :
    HRel.law_fix_stream (causalC L mem pacing) := by
  intro h ℓ α _ ord ret d d' hd b₁ b₂ hb
  cases (rfl : d = d')
  exact causal_fix_stream d
    (fun h' hh' x y hxy => hb h' hh' x y
      (fun h'' hh'' => SAgree.mono hh'' hxy))

theorem causal_law_fix_tick :
    HRel.law_fix_tick (causalC L mem pacing) := by
  intro h ℓ σ d d' hd b₁ b₂ hb
  cases (rfl : d = d')
  exact causal_fix_tick d
    (fun h' hh' x y hxy => hb h' hh' x y
      (fun h'' hh'' => TAgree.mono hh'' hxy))

/-- The step-causality laws — one obligation per signature op, in
declaration order. -/
theorem causalLaws : HRel.Laws (causalC L mem pacing) :=
  ⟨by causal_law_tac, -- map
   by causal_law_tac, -- filterMap
   by causal_law_tac, -- broadcast
   by causal_law_tac, -- demux
   by causal_law_tac, -- values
   by causal_law_tac, -- weaken_retries
   by causal_law_tac, -- union
   by causal_law_tac, -- assume_ordering
   by causal_law_tac, -- fold
   by causal_law_tac, -- fold_monotone
   by causal_law_tac, -- snapshot
   by causal_law_tac, -- batch
   by causal_law_tac, -- batch_ordered
   by causal_law_tac, -- assume_ordering_batch
   by causal_law_tac, -- mapBatchWith
   by causal_law_tac, -- mapBatch
   by causal_law_tac, -- mapBatchesWith
   by causal_law_tac, -- scan_batches_across_ticks
   by causal_law_tac, -- fold_batches_across_ticks_monotone
   by causal_law_tac, -- sample_every
   by causal_law_tac, -- timeout_snapshot
   by causal_law_tac, -- source_interval_batch
   by causal_law_tac, -- scan_batches_unordered_across_ticks
   by causal_law_tac, -- scan_batches_unordered
   by causal_law_tac, -- scan_batches_unordered₂
   by causal_law_tac, -- scan_across_ticks
   by causal_law_tac, -- mapTick
   by causal_law_tac, -- zipTick
   by causal_law_tac, -- fold_across_ticks_monotone
   by causal_law_tac, -- mapMonotone
   by causal_law_tac, -- forgetBound
   by causal_law_tac, -- defer
   by causal_law_tac, -- filterMapBatchesWith
   by causal_law_tac, -- mapBatchesUnordered
   by causal_law_tac, -- emitMultisetBatches
   by causal_law_tac, -- allTicks
   by causal_law_tac, -- emitBatches
   by causal_law_tac, -- emitBatchesUnordered
   causal_law_fix_stream,
   causal_law_fix_tick⟩

end HydroV2
