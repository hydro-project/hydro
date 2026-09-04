import HydroV2.Values

/-!
# HydroV2 · `ReaderSem` — the correlated-nondeterminism reader

`Values` lifted pointwise over an abstract decision environment `D`:
carriers are `D → (Values carrier)`, decision arguments are **lenses**
`D → (Values decision)`. Composition is the reader monad's, so a
diamond (one wire consumed twice) shares its reader — both consumers
see the *same* denotational run at every `d`: correlation by shared
input, no program induction.

The payoff is the **naming identity**: for a concrete program `P` and
honest field-accessor lenses, `(P@ReaderSem lenses) d` beta-reduces to
`P@Values (fields of d)` — definitionally (`rfl`) — which is what lets
a `∀ d`-quantified denotational contract apply to the reader leg of a
coupled run at *every* environment, no witness naming required.
-/

namespace HydroV2

set_option warn.classDefReducibility false in
/-- `Values` lifted pointwise over a decision environment `D`. -/
def ReaderSem (L : Type) (mem : L → Nat) (D : Type) : HydroSem L mem where
  Stream ℓ α _ ord ret := D → (Values L mem).Stream ℓ α ord ret
  KeyedStream p c α _ ord ret :=
    D → (Values L mem).KeyedStream p c α ord ret
  Singleton ℓ α σ _ ord ret b :=
    D → (Values L mem).Singleton ℓ α σ ord ret b
  TickSingleton ℓ σ b := D → (Values L mem).TickSingleton ℓ σ b
  TickStream ℓ α _ ord ret := D → (Values L mem).TickStream ℓ α ord ret
  TransportDec _ _ := Unit
  OrderSelDec n α := D → OrderSelection n α
  SnapDec n α ord := D → SnapshotCuts n α ord
  BatchDec n α := D → BatchCuts n α
  OrdBatchDec n := D → OrderedBatchCuts n
  BatchOrdSelDec n α := D → BatchOrderSelection n α
  SampleDec n := D → SampleTimes n
  TimerDec n := D → TimerVerdicts n
  PulseDec n := D → TimingPulses n
  EmitDec _ _ := Unit
  FixDec := D → UnfoldFuel
  map s f := fun d => (Values L mem).map (s d) f
  filterMap s f := fun d => (Values L mem).filterMap (s d) f
  broadcast _dt s := fun d => (Values L mem).broadcast () (s d)
  demux _dt s addr := fun d => (Values L mem).demux () (s d) addr
  values k := fun d => (Values L mem).values (k d)
  weaken_retries s := fun d => (Values L mem).weaken_retries (s d)
  union a b := fun d => (Values L mem).union (a d) (b d)
  assume_ordering u sel := fun d =>
    (Values L mem).assume_ordering (u d) (sel d)
  fold g init ok s := fun d => (Values L mem).fold g init ok (s d)
  fold_monotone vo g init ok hinfl s := fun d =>
    (Values L mem).fold_monotone vo g init ok hinfl (s d)
  snapshot s cut := fun d => (Values L mem).snapshot (s d) (cut d)
  batch s cut := fun d => (Values L mem).batch (s d) (cut d)
  batch_ordered s cut := fun d =>
    (Values L mem).batch_ordered (s d) (cut d)
  assume_ordering_batch bs sel := fun d =>
    (Values L mem).assume_ordering_batch (bs d) (sel d)
  mapBatchWith bs t f := fun d =>
    (Values L mem).mapBatchWith (bs d) (t d) f
  mapBatch bs f := fun d => (Values L mem).mapBatch (bs d) f
  mapBatchesWith bs t f := fun d =>
    (Values L mem).mapBatchesWith (bs d) (t d) f
  filterMapBatchesWith bs t f := fun d =>
    (Values L mem).filterMapBatchesWith (bs d) (t d) f
  scan_batches_across_ticks bs t g init := fun d =>
    (Values L mem).scan_batches_across_ticks (bs d) (t d) g init
  fold_batches_across_ticks_monotone vo g init comm hinfl bs := fun d =>
    (Values L mem).fold_batches_across_ticks_monotone vo g init comm
      hinfl (bs d)
  scan_batches_unordered_across_ticks bs t g init := fun d =>
    (Values L mem).scan_batches_unordered_across_ticks (bs d) (t d)
      g init
  scan_batches_unordered bs g init := fun d =>
    (Values L mem).scan_batches_unordered (bs d) g init
  scan_batches_unordered₂ bs cs g init := fun d =>
    (Values L mem).scan_batches_unordered₂ (bs d) (cs d) g init
  scan_across_ticks t g init := fun d =>
    (Values L mem).scan_across_ticks (t d) g init
  sample_every t times := fun d =>
    (Values L mem).sample_every (t d) (times d)
  timeout_snapshot s verd := fun d =>
    (Values L mem).timeout_snapshot (s d) (verd d)
  source_interval_batch pulses := fun d =>
    (Values L mem).source_interval_batch (pulses d)
  mapTick s f := fun d => (Values L mem).mapTick (s d) f
  zipTick a b := fun d => (Values L mem).zipTick (a d) (b d)
  fold_across_ticks_monotone vo g init hinfl s := fun d =>
    (Values L mem).fold_across_ticks_monotone vo g init hinfl (s d)
  mapMonotone vo' m h hpres := fun d =>
    (Values L mem).mapMonotone vo' (m d) h hpres
  forgetBound m := fun d => (Values L mem).forgetBound (m d)
  defer init t := fun d => (Values L mem).defer init (t d)
  allTicks bs := fun d => (Values L mem).allTicks (bs d)
  mapBatchesUnordered bs t f := fun d =>
    (Values L mem).mapBatchesUnordered (bs d) (t d) f
  emitBatches t := fun d => (Values L mem).emitBatches (t d)
  emitMultisetBatches t _e := fun d =>
    (Values L mem).emitMultisetBatches (t d) ()
  emitBatchesUnordered t := fun d =>
    (Values L mem).emitBatchesUnordered (t d)
  fix_stream df body := fun d =>
    (Values L mem).fix_stream (df d) (fun x => body (fun _ => x) d)
  fix_tick df body := fun d =>
    (Values L mem).fix_tick (df d) (fun x => body (fun _ => x) d)

end HydroV2
