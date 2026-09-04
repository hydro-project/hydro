import HydroV2.Values

/-!
# HydroV2 · the relational packaging (`RelSem`)

The ∃-packaging of the graded denotation: carriers are **sets of
`Values` runs**, and every decision type is `Unit` — the denotational
nondeterminism is absorbed into the carrier. Each op is, definitionally,
the **image of the corresponding `Values` op over its decision space**,
so composition is the nondeterminism (powerset) monad's bind: a whole
program instantiated at `RelSem` denotes the *compositionally-defined
set of its denotational runs*, with no induction over program syntax
anywhere.

Faithfulness to `Values` is therefore per-op and definitional: to audit
that `RelSem` adds or drops nothing, read each field — it is exactly
`{y | ∃ inputs ∈ carriers, ∃ decision, y = Values-op inputs decision}`.

**Decorrelation caveat**: multi-input ops take independent ∃ per input,
so diamond-shaped reuse of a wire is decorrelated — `RelSem` is the
free/possibilistic interpretation, a sound over-approximation of the
set of genuine whole-program runs. Transfer statements against `RelSem`
therefore quantify over at least every genuine denotational run.

Cycles: `fix_*` is the union over all unfolding depths of iterating the
set-level body from the singleton bottom — the image of `Values`'s
fuel-indexed Kleene iterate over its fuel decision.
-/

namespace HydroV2

set_option warn.classDefReducibility false in
/-- The relational (set-valued) interpretation: sets of `Values` runs,
decisions absorbed. -/
def RelSem (L : Type) (mem : L → Nat) : HydroSem L mem where
  Stream ℓ α _ ord ret := Set ((Values L mem).Stream ℓ α ord ret)
  KeyedStream p c α _ ord ret :=
    Set ((Values L mem).KeyedStream p c α ord ret)
  Singleton ℓ α σ _ ord ret b :=
    Set ((Values L mem).Singleton ℓ α σ ord ret b)
  TickSingleton ℓ σ b := Set ((Values L mem).TickSingleton ℓ σ b)
  TickStream ℓ α _ ord ret := Set ((Values L mem).TickStream ℓ α ord ret)
  TransportDec _ _ := Unit
  OrderSelDec _ _ := Unit
  SnapDec _ _ _ := Unit
  BatchDec _ _ := Unit
  OrdBatchDec _ := Unit
  BatchOrdSelDec _ _ := Unit
  SampleDec _ := Unit
  TimerDec _ := Unit
  PulseDec _ := Unit
  EmitDec _ _ := Unit
  FixDec := Unit
  map s f := {y | ∃ x ∈ s, y = (Values L mem).map x f}
  filterMap s f := {y | ∃ x ∈ s, y = (Values L mem).filterMap x f}
  broadcast _d s := {y | ∃ x ∈ s, y = (Values L mem).broadcast () x}
  demux _d s addr :=
    {y | ∃ x ∈ s, y = (Values L mem).demux () x addr}
  values k := {y | ∃ x ∈ k, y = (Values L mem).values x}
  weaken_retries s := {y | ∃ x ∈ s, y = (Values L mem).weaken_retries x}
  union a b :=
    {y | ∃ xa ∈ a, ∃ xb ∈ b, y = (Values L mem).union xa xb}
  assume_ordering u _d :=
    {y | ∃ x ∈ u, ∃ d, y = (Values L mem).assume_ordering x d}
  fold g init ok s := {y | ∃ x ∈ s, y = (Values L mem).fold g init ok x}
  fold_monotone vo g init ok hinfl s :=
    {y | ∃ x ∈ s, y = (Values L mem).fold_monotone vo g init ok hinfl x}
  snapshot s _d := {y | ∃ x ∈ s, ∃ d, y = (Values L mem).snapshot x d}
  batch s _d := {y | ∃ x ∈ s, ∃ d, y = (Values L mem).batch x d}
  batch_ordered s _d :=
    {y | ∃ x ∈ s, ∃ d, y = (Values L mem).batch_ordered x d}
  assume_ordering_batch bs _d :=
    {y | ∃ x ∈ bs, ∃ d, y = (Values L mem).assume_ordering_batch x d}
  mapBatchWith bs t f :=
    {y | ∃ xb ∈ bs, ∃ xt ∈ t, y = (Values L mem).mapBatchWith xb xt f}
  mapBatch bs f := {y | ∃ x ∈ bs, y = (Values L mem).mapBatch x f}
  mapBatchesWith bs t f :=
    {y | ∃ xb ∈ bs, ∃ xt ∈ t,
      y = (Values L mem).mapBatchesWith xb xt f}
  scan_batches_across_ticks bs t g init :=
    {y | ∃ xb ∈ bs, ∃ xt ∈ t,
      y = (Values L mem).scan_batches_across_ticks xb xt g init}
  fold_batches_across_ticks_monotone vo g init comm hinfl bs :=
    {y | ∃ x ∈ bs, y = (Values L mem).fold_batches_across_ticks_monotone
      vo g init comm hinfl x}
  sample_every t _d :=
    {y | ∃ x ∈ t, ∃ d, y = (Values L mem).sample_every x d}
  timeout_snapshot s _d :=
    {y | ∃ x ∈ s, ∃ d, y = (Values L mem).timeout_snapshot x d}
  source_interval_batch _d :=
    {y | ∃ d, y = (Values L mem).source_interval_batch d}
  scan_batches_unordered_across_ticks bs t g init :=
    {y | ∃ xb ∈ bs, ∃ xt ∈ t, y =
      (Values L mem).scan_batches_unordered_across_ticks xb xt g init}
  scan_batches_unordered bs g init :=
    {y | ∃ x ∈ bs,
      y = (Values L mem).scan_batches_unordered x g init}
  scan_batches_unordered₂ bs cs g init :=
    {y | ∃ xb ∈ bs, ∃ xc ∈ cs,
      y = (Values L mem).scan_batches_unordered₂ xb xc g init}
  scan_across_ticks t g init :=
    {y | ∃ x ∈ t, y = (Values L mem).scan_across_ticks x g init}
  mapTick s f := {y | ∃ x ∈ s, y = (Values L mem).mapTick x f}
  zipTick a b :=
    {y | ∃ xa ∈ a, ∃ xb ∈ b, y = (Values L mem).zipTick xa xb}
  fold_across_ticks_monotone vo g init hinfl s :=
    {y | ∃ x ∈ s,
      y = (Values L mem).fold_across_ticks_monotone vo g init hinfl x}
  mapMonotone vo' m h hpres :=
    {y | ∃ x ∈ m, y = (Values L mem).mapMonotone vo' x h hpres}
  forgetBound m := {y | ∃ x ∈ m, y = (Values L mem).forgetBound x}
  defer init t := {y | ∃ x ∈ t, y = (Values L mem).defer init x}
  filterMapBatchesWith bs t f :=
    {y | ∃ xb ∈ bs, ∃ xt ∈ t,
      y = (Values L mem).filterMapBatchesWith xb xt f}
  mapBatchesUnordered bs t f :=
    {y | ∃ xb ∈ bs, ∃ xt ∈ t,
      y = (Values L mem).mapBatchesUnordered xb xt f}
  emitMultisetBatches t _d :=
    {y | ∃ x ∈ t, y = (Values L mem).emitMultisetBatches x ()}
  allTicks bs := {y | ∃ x ∈ bs, y = (Values L mem).allTicks x}
  emitBatches t := {y | ∃ x ∈ t, y = (Values L mem).emitBatches x}
  emitBatchesUnordered t :=
    {y | ∃ x ∈ t, y = (Values L mem).emitBatchesUnordered x}
  fix_stream {_ℓ _α _ ord ret} _d body :=
    {y | ∃ k, y ∈ iterate body {v | v = fun _i => PoolBot ord ret} k}
  fix_tick _d body :=
    {y | ∃ k, y ∈ iterate body {v | v = fun _i => ([] : Trace _)} k}

end HydroV2
