import Hydro.HydroRelLaws
import Hydro.EagerProj

/-!
# Hydro · the eager-agreement relation (`HRel` instance)

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

namespace Hydro

/-- `.den` of a graded singleton cell pack, uniformly over the
bound. -/
def ESing.den {n : Nat} {α σ : Type} [DecidableEq α] {ord : StrOrd} :
    ∀ {b : SingBound σ}, ESing n α σ ord b → SingletonV n α σ ord b
  | .unbounded, e => EPack.den e
  | .monotonic _, e => EPack.den e

/-- `.den` of a ticked pack. -/
abbrev ETick.den {n : Nat} {σ : Type} (e : ETick n σ) : TickV n σ :=
  EPack.den e

/-- The eager-agreement relation families: denotation equality on
carriers, equality on (shared-vocabulary) decisions. -/
def eagC (L : Type) (mem : L → Nat) :
    HRelC Unit (Eager L mem) (Values L mem) where
  streamRel _ e v := e.den = v
  keyedRel _ e v := e.den = v
  singRel _ e v := e.den = v
  tickedRel _ e v := e.den = v
  -- the eager data leg runs on the denotation's quotients (for now)
  boundedRel _ b v := b = v
  bsingRel _ x y := x = y
  tickedBoundedRel _ e v := e.den = v
  tickStreamRel _ e v := e.den = v
  transportRel _ d d' := d = d'
  orderSelRel _ d d' := d = d'
  snapRel _ d d' := d = d'
  batchRel _ d d' := d = d'
  ordBatchRel _ d d' := d = d'
  sampleRel _ d d' := d = d'
  timerRel _ d d' := d = d'
  pulseRel _ d d' := d = d'
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
    all_goals (try (rename_i hf; have := funext hf; subst this))
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

theorem eagC_boundedOfRel_iff : (sh : TickShape) →
    (x : BoundedOf (Eager L mem).BoundedSingleton (Eager L mem).BoundedStream sh) →
    (y : BoundedOf (Values L mem).BoundedSingleton (Values L mem).BoundedStream sh) →
    ((eagC L mem).boundedOfRel () sh x y ↔ x = y)
  | .sing _, _, _ => Iff.rfl
  | .stream _ _ _ _, _, _ => Iff.rfl
  | .pair a b, x, y => by
    show ((eagC L mem).boundedOfRel () a x.1 y.1 ∧ (eagC L mem).boundedOfRel () b x.2 y.2)
      ↔ x = y
    rw [eagC_boundedOfRel_iff a, eagC_boundedOfRel_iff b]
    exact ⟨fun h => Prod.ext h.1 h.2, fun h => ⟨congrArg Prod.fst h, congrArg Prod.snd h⟩⟩

theorem eagC_tickedOfRel_iff {ℓ : L} : (sh : TickShape) →
    (x : TickedOf (Eager L mem).Ticked (Eager L mem).TickStream ℓ sh) →
    (y : TickedOf (Values L mem).Ticked (Values L mem).TickStream ℓ sh) →
    ((eagC L mem).tickedOfRel () sh x y ↔ EagerTick.den sh x = y)
  | .sing _, _, _ => Iff.rfl
  | .stream _ _ _ _, _, _ => Iff.rfl
  | .pair a b, x, y => by
    show ((eagC L mem).tickedOfRel () a x.1 y.1 ∧ (eagC L mem).tickedOfRel () b x.2 y.2)
      ↔ (EagerTick.den a x.1, EagerTick.den b x.2) = y
    rw [eagC_tickedOfRel_iff a, eagC_tickedOfRel_iff b]
    exact ⟨fun h => Prod.ext h.1 h.2, fun h => ⟨congrArg Prod.fst h, congrArg Prod.snd h⟩⟩

theorem eag_law_tick_scan : HRel.law_tick_scan (eagC L mem) := by
  intro i ℓ sts ins outs x x' hx g g' hg init
  have hgg : g = g' := funext fun j => funext fun s => funext fun inp => by
    have := hg j s s ((eagC_boundedOfRel_iff sts s s).2 rfl) inp inp
      ((eagC_boundedOfRel_iff ins inp inp).2 rfl)
    exact Prod.ext ((eagC_boundedOfRel_iff sts _ _).1 this.1)
      ((eagC_boundedOfRel_iff outs _ _).1 this.2)
  subst hgg
  have hx' := (eagC_tickedOfRel_iff ins x x').1 hx
  subst hx'
  exact (eagC_tickedOfRel_iff outs _ _).2 (EagerTick.den_pack outs _ _ _)

/-- The eager-agreement laws — one obligation per signature op, in
declaration order. -/
theorem eagLaws : HRel.Laws (eagC L mem) :=
  ⟨by eag_law_tac, -- map
   by eag_law_tac, -- filterMap
   by eag_law_tac, -- broadcast_closed
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
   by eag_law_tac, -- sample_every
   by eag_law_tac, -- timeout_snapshot
   by eag_law_tac, -- source_interval_batch
   by eag_law_tac, -- mapTick
   by eag_law_tac, -- zipTick
   by eag_law_tac, -- defer_tick
   by eag_law_tac, -- allTicks
   by eag_law_tac, -- flattenOrdered
   by eag_law_tac, -- flattenUnordered
   eag_law_fix_stream,
   eag_law_fix_tick,
   by eag_law_tac, -- bmap
   by eag_law_tac, -- bfilterMap
   by eag_law_tac, -- bflatMapOrdered
   by eag_law_tac, -- bflatMapUnordered
   by eag_law_tac, -- bofList
   by eag_law_tac, -- bcount
   by eag_law_tac, -- bfold
   by eag_law_tac, -- benumerate
   by eag_law_tac, -- bfirst
   by eag_law_tac, -- bcrossSingleton
   by eag_law_tac, -- bchain
   by eag_law_tac, -- bweakenOrder
   by eag_law_tac, -- bfilter
   by eag_law_tac, -- bkeyedFold
   by eag_law_tac, -- bkeys
   by eag_law_tac, -- bjoin
   by eag_law_tac, -- bantiJoin
   by eag_law_tac, -- bfilterNotIn
   by eag_law_tac, -- bmax
   by eag_law_tac, -- bfilterIf
   by eag_law_tac, -- bsPure
   by eag_law_tac, -- bsMap
   by eag_law_tac, -- bsZip
   by eag_law_tac, -- boMap
   by eag_law_tac, -- boUnwrapOr
   by eag_law_tac, -- boFilter
   by eag_law_tac, -- boIsSome
   eag_law_tick_scan⟩

end Hydro
