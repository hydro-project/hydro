import Hydro.HydroRelLaws
import Hydro.SchedCausal

/-!
# Hydro · the step-causality relation (`HRel` instance)

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

namespace Hydro

/-- The step-causality relation families: agreement below the horizon
at the machine diagonal (decisions equal). -/
def causalC (L : Type) (mem : L → Nat)
    (pacing : Pacing L mem) :
    HRelC Nat (SchedSem L mem pacing) (SchedSem L mem pacing) where
  streamRel h x y := SAgree h x y
  keyedRel h x y := SAgree h x y
  singRel h x y := FAgree h x y
  ksingRel h x y := FAgree h x y
  atomicRel h x y := TAgree h x y
  -- one tick's content at the machine: the concrete lists, equal
  boundedRel _ x y := x = y
  bsingRel _ x y := x = y
  atomicBoundedRel h x y := TAgree h x y
  atomicStreamRel h x y := TAgree h x y
  bkeyedRel _ x y := x = y
  bksingRel _ x y := x = y
  atomicKSingRel h x y := TAgree h x y
  atomicKeyedRel h x y := TAgree h x y
  transportRel _ d d' := d = d'
  orderSelRel _ d d' := d = d'
  tickOrderSelRel _ d d' := d = d'
  snapRel _ d d' := d = d'
  batchRel _ d d' := d = d'
  intervalRel _ d d' := d = d'
  ticksRel _ d d' := d = d'
  clockRel _ d d' := d = d'
  fixRel _ d d' := d = d'

variable {L : Type} {mem : L → Nat}
  {pacing : Pacing L mem}

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
      | (apply causal_broadcast_closed <;> assumption)
      | (apply causal_demux <;> assumption)
      | (apply causal_values <;> assumption)
      | (apply causal_into_keyed <;> assumption)
      | (apply causal_kassume_ordering <;> assumption)
      | (apply causal_kfold_early_stop <;> assumption)
      | (apply causal_kget_max_key <;> assumption)
      | (apply causal_tinto_keyed <;> assumption)
      | (apply causal_tassume_ordering <;> assumption)
      | (apply causal_reduce_watermark <;> assumption)
      | (apply causal_weaken_retries <;> assumption)
      | (apply causal_union <;> assumption)
      | (apply causal_assume_ordering <;> assumption)
      | (apply causal_fold <;> assumption)
      | (apply causal_fold_monotone <;> assumption)
      | (apply causal_snapshot <;> assumption)
      | (apply causal_batch <;> assumption)
      | (apply causal_source_interval <;> assumption)
      | (apply causal_firstTick <;> assumption)
      | (apply causal_mapTick <;> assumption)
      | (apply causal_zipTick <;> assumption)
      | (apply causal_defer_tick <;> assumption)
      | (apply causal_allTicks <;> assumption)
      | (apply causal_flattenOrdered <;> assumption)
      | (apply causal_flattenUnordered <;> assumption)
      | (apply causal_kentries <;> assumption)
      | (apply causal_kmap <;> assumption)
      | (apply causal_kallTicks <;> assumption)
      | (apply causal_kfold <;> assumption)
      | (apply causal_ksnapshot <;> assumption)
      | (apply causal_tfold <;> assumption)
      | (apply causal_latest <;> assumption)
      | (apply causal_latest_monotone <;> assumption)
      | (apply causal_time <;> assumption)
      | (apply causal_snapshot_bounded <;> assumption)
      -- in-tick operators: one tick's content is related by equality
      -- (closures pointwise), wires by step agreement
      | rfl
      | assumption
      | (rename_i hf; have hfe := funext hf; subst hfe; rfl)))

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

theorem causalC_boundedOfRel_iff : (sh : TickShape) →
    (x y : BoundedOf (SchedSem L mem pacing).BoundedSingleton
      (SchedSem L mem pacing).BoundedStream (SchedSem L mem pacing).BoundedKeyedSingleton
      (SchedSem L mem pacing).BoundedKeyedStream sh) →
    ((causalC L mem pacing).boundedOfRel 0 sh x y ↔ x = y)
  | .sing _, _, _ => Iff.rfl
  | .stream _ _ _ _, _, _ => Iff.rfl
  | .ksing _ _ _ _, _, _ => Iff.rfl
  | .kstream _ _ _ _ _ _, _, _ => Iff.rfl
  | .pair a b, x, y => by
    show ((causalC L mem pacing).boundedOfRel 0 a x.1 y.1
        ∧ (causalC L mem pacing).boundedOfRel 0 b x.2 y.2) ↔ x = y
    rw [causalC_boundedOfRel_iff a, causalC_boundedOfRel_iff b]
    exact ⟨fun h => Prod.ext h.1 h.2, fun h => ⟨congrArg Prod.fst h, congrArg Prod.snd h⟩⟩

theorem causalC_boundedOfRel_index (h : Nat) (sh : TickShape)
    (x y : BoundedOf (SchedSem L mem pacing).BoundedSingleton
      (SchedSem L mem pacing).BoundedStream (SchedSem L mem pacing).BoundedKeyedSingleton
      (SchedSem L mem pacing).BoundedKeyedStream sh) :
    (causalC L mem pacing).boundedOfRel h sh x y
      ↔ (causalC L mem pacing).boundedOfRel 0 sh x y := by
  induction sh with
  | sing _ => exact Iff.rfl
  | stream _ _ _ _ => exact Iff.rfl
  | ksing _ _ _ _ => exact Iff.rfl
  | kstream _ _ _ _ _ _ => exact Iff.rfl
  | pair a b iha ihb => exact and_congr (iha _ _) (ihb _ _)

theorem causalC_atomicOfRel_iff {ℓ : L} (h : Nat) : (sh : TickShape) →
    (x y : AtomicOf (SchedSem L mem pacing).Atomic
      (SchedSem L mem pacing).AtomicStream (SchedSem L mem pacing).AtomicKeyedSingleton
      (SchedSem L mem pacing).AtomicKeyedStream ℓ sh) →
    ((causalC L mem pacing).atomicOfRel h sh x y ↔ TAgreeOf h sh x y)
  | .sing _, _, _ => Iff.rfl
  | .stream _ _ _ _, _, _ => Iff.rfl
  | .ksing _ _ _ _, _, _ => Iff.rfl
  | .kstream _ _ _ _ _ _, _, _ => Iff.rfl
  | .pair a b, x, y => by
    show ((causalC L mem pacing).atomicOfRel h a x.1 y.1
        ∧ (causalC L mem pacing).atomicOfRel h b x.2 y.2)
      ↔ (TAgreeOf h a x.1 y.1 ∧ TAgreeOf h b x.2 y.2)
    rw [causalC_atomicOfRel_iff h a, causalC_atomicOfRel_iff h b]

theorem causal_law_tick_scan : HRel.law_tick_scan (causalC L mem pacing) := by
  intro h ℓ sts ins outs x x' hx g g' hg init
  have hgg : g = g' := funext fun j => funext fun s => funext fun inp => by
    have := hg j s s
      ((causalC_boundedOfRel_index h sts s s).2
        ((causalC_boundedOfRel_iff sts s s).2 rfl)) inp inp
      ((causalC_boundedOfRel_index h ins inp inp).2
        ((causalC_boundedOfRel_iff ins inp inp).2 rfl))
    exact Prod.ext ((causalC_boundedOfRel_iff sts _ _).1
      ((causalC_boundedOfRel_index h sts _ _).1 this.1))
      ((causalC_boundedOfRel_iff outs _ _).1
      ((causalC_boundedOfRel_index h outs _ _).1 this.2))
  subst hgg
  exact (causalC_atomicOfRel_iff h outs _ _).2
    (causal_tick_scan sts ins outs g init ((causalC_atomicOfRel_iff h ins x x').1 hx))

/-- The step-causality laws — one obligation per signature op, in
declaration order. -/
theorem causalLaws : HRel.Laws (causalC L mem pacing) :=
  ⟨by causal_law_tac, -- map
   by causal_law_tac, -- filterMap
   by causal_law_tac, -- broadcast_closed
   by causal_law_tac, -- demux
   by causal_law_tac, -- values
   by causal_law_tac, -- into_keyed
   by causal_law_tac, -- kassume_ordering
   by causal_law_tac, -- kfold_early_stop
   by causal_law_tac, -- kget_max_key
   by causal_law_tac, -- weaken_retries
   by causal_law_tac, -- union
   by causal_law_tac, -- assume_ordering
   by causal_law_tac, -- fold
   by causal_law_tac, -- fold_monotone
   by causal_law_tac, -- snapshot
   by causal_law_tac, -- batch
   by causal_law_tac, -- source_interval
   by causal_law_tac, -- firstTick
   by causal_law_tac, -- mapTick
   by causal_law_tac, -- zipTick
   by causal_law_tac, -- defer_tick
   by causal_law_tac, -- allTicks
   by causal_law_tac, -- flattenOrdered
   by causal_law_tac, -- flattenUnordered
   causal_law_fix_stream,
   causal_law_fix_tick,
   by causal_law_tac, -- tinto_keyed
   by causal_law_tac, -- tassume_ordering
   by causal_law_tac, -- reduce_watermark
   by causal_law_tac, -- kentries
   by causal_law_tac, -- kmap
   by causal_law_tac, -- kallTicks
   by causal_law_tac, -- kfold
   by causal_law_tac, -- ksnapshot
   by causal_law_tac, -- tfold
   by causal_law_tac, -- latest
   by causal_law_tac, -- latest_monotone
   by causal_law_tac, -- time
   by causal_law_tac, -- snapshot_bounded
   by causal_law_tac, -- bmap
   by causal_law_tac, -- bfilterMap
   by causal_law_tac, -- bflatMapOrdered
   by causal_law_tac, -- bflatMapUnordered
   by causal_law_tac, -- bofList
   by causal_law_tac, -- bcount
   by causal_law_tac, -- bisEmpty
   by causal_law_tac, -- bfold
   by causal_law_tac, -- benumerate
   by causal_law_tac, -- bfirst
   by causal_law_tac, -- bcrossSingleton
   by causal_law_tac, -- bchain
   by causal_law_tac, -- bweakenOrder
   by causal_law_tac, -- bfilter
   by causal_law_tac, -- binto_keyed
   by causal_law_tac, -- bkfold
   by causal_law_tac, -- bksFilter
   by causal_law_tac, -- bksMap
   by causal_law_tac, -- bksKeys
   by causal_law_tac, -- bksEntries
   by causal_law_tac, -- bjoin
   by causal_law_tac, -- bantiJoin
   by causal_law_tac, -- bfilterNotIn
   by causal_law_tac, -- bmax
   by causal_law_tac, -- bfilterIf
   by causal_law_tac, -- bsPure
   by causal_law_tac, -- bsMap
   by causal_law_tac, -- bsZip
   by causal_law_tac, -- boMap
   by causal_law_tac, -- boUnwrapOr
   by causal_law_tac, -- boOr
   by causal_law_tac, -- boFilterIf
   by causal_law_tac, -- boFilter
   by causal_law_tac, -- boIsSome
   by causal_law_tac, -- boIsNone
   by causal_law_tac, -- boZip
   by causal_law_tac, -- boFilterMap
   by causal_law_tac, -- boIntoStream
   by causal_law_tac, -- bsFilterIf
   by causal_law_tac, -- bsFilterMap
   by causal_law_tac, -- boZipSingleton
   by causal_law_tac, -- bsZipOptional
   by causal_law_tac, -- bmin
   by causal_law_tac, -- breduce
   by causal_law_tac, -- bsort
   by causal_law_tac, -- bcrossOptional
   by causal_law_tac, -- bkmap
   by causal_law_tac, -- bkcrossSingleton
   by causal_law_tac, -- bkchain
   by causal_law_tac, -- bkweakenOrder
   by causal_law_tac, -- bkreduce
   by causal_law_tac, -- bksJoin
   by causal_law_tac, -- bksFilterKeyNotIn
   by causal_law_tac, -- bksKeyCount
   by causal_law_tac, -- bksIntoStream
   causal_law_tick_scan⟩

end Hydro
