import HydroV2.Eager

/-!
# HydroV2 · generic eager-projection lemmas (`EagerProj`)

The per-operator identities pinning `Eager` to `Values`: for every op,
the denotation leg of the materialized op IS the `Values` op on the
inputs' denotation legs. Each lemma is `rfl` **at variable arguments**
(the ops are defined that way — `Eager.lean` never re-spells a
formula), so the projection identity of a whole program assembles by
`eager_transfer [<its def names>]` — a `simp only` over this set with a
`with_reducible rfl` closer, at elaborator cost only. Combined with the
carrier-level `agree` field, a program's executed data is **provably**
its `Values` denotation, and the two interpretations cannot silently
diverge as either evolves (the lemma or the transfer breaks loudly).

Genericity accounting: everything here is per-OPERATOR; a program pays
one `eager_transfer` call naming its defs.
-/

namespace HydroV2

variable {L : Type} {mem : L → Nat} {ℓ : L}

/-! ## Instance-typed embeds and inputs

The carriers have two spellings (`EPack …` raw vs
`(Eager L mem).Stream …` instance-projected) that are **not reducibly
equal**, and `simp` matches at reducible transparency — one raw-spelled
node blinds every rewrite above it (FINDINGS D33). Everything the
transfer set touches is therefore typed AT the instance projections. -/

/-- Embed a `Values` stream (the fix-body seam; proof-side only). -/
def EagStream.embedV {α : Type} [DecidableEq α] {ord : StrOrd}
    {ret : Retries} (v : (Values L mem).Stream ℓ α ord ret) :
    (Eager L mem).Stream ℓ α ord ret := EPack.ofDen v

/-- Embed a `Values` tick singleton (the `fix_tick` seam). -/
def EagTickSing.embedV {σ : Type}
    (v : (Values L mem).TickSingleton ℓ σ .unbounded) :
    (Eager L mem).TickSingleton ℓ σ .unbounded := EPack.ofDen v

/-- A materialized stream input (executables). -/
def EagStream.input {α : Type} [DecidableEq α] {ord : StrOrd}
    {ret : Retries} (d : Vector (PoolCarrier α ord ret) (mem ℓ)) :
    (Eager L mem).Stream ℓ α ord ret := EPack.ofData d

/-- A materialized tick-singleton input (executables). -/
def EagTickSing.input {σ : Type} (d : Vector (Trace σ) (mem ℓ)) :
    (Eager L mem).TickSingleton ℓ σ .unbounded := EPack.ofData d

@[simp] theorem eag_stream_embedV_den {α : Type} [DecidableEq α]
    {ord : StrOrd} {ret : Retries}
    (v : (Values L mem).Stream ℓ α ord ret) :
    (EagStream.embedV (ℓ := ℓ) v).den = v := rfl

@[simp] theorem eag_ticksing_embedV_den {σ : Type}
    (v : (Values L mem).TickSingleton ℓ σ .unbounded) :
    (EagTickSing.embedV (ℓ := ℓ) v).den = v := rfl

@[simp] theorem eag_stream_input_den {α : Type} [DecidableEq α]
    {ord : StrOrd} {ret : Retries}
    (d : Vector (PoolCarrier α ord ret) (mem ℓ)) :
    (EagStream.input (ℓ := ℓ) d).den = fun i => d.get i := rfl

@[simp] theorem eag_ticksing_input_den {σ : Type}
    (d : Vector (Trace σ) (mem ℓ)) :
    (EagTickSing.input (ℓ := ℓ) d).den = fun i => d.get i := rfl

/-! ## Per-op projections (all `rfl` at variable arguments) -/

theorem eag_map_den {α β : Type} [DecidableEq α] [DecidableEq β]
    {ord : StrOrd} (s : (Eager L mem).Stream ℓ α ord .exactlyOnce)
    (f : Fin (mem ℓ) → α → β) :
    ((Eager L mem).map s f).den = (Values L mem).map s.den f := rfl

theorem eag_filterMap_den {α β : Type} [DecidableEq α] [DecidableEq β]
    {ord : StrOrd} (s : (Eager L mem).Stream ℓ α ord .exactlyOnce)
    (f : Fin (mem ℓ) → α → Option β) :
    ((Eager L mem).filterMap s f).den
      = (Values L mem).filterMap s.den f := rfl

theorem eag_broadcast_den {c p : L} {α : Type} [DecidableEq α]
    {ord : StrOrd} {ret : Retries}
    (d : (Values L mem).TransportDec (mem p) (mem c))
    (s : (Eager L mem).Stream c α ord ret) :
    ((Eager L mem).broadcast (p := p) d s).den
      = (Values L mem).broadcast d s.den := rfl

theorem eag_demux_den {c p : L} {α : Type} [DecidableEq α] {ord : StrOrd}
    (d : (Values L mem).TransportDec (mem p) (mem c))
    (s : (Eager L mem).Stream c (Nat × α) ord .exactlyOnce)
    (addr : Fin (mem p) → Nat) :
    ((Eager L mem).demux d s addr).den
      = (Values L mem).demux d s.den addr := rfl

theorem eag_values_den {p c : L} {α : Type} [DecidableEq α]
    {ord : StrOrd} {ret : Retries}
    (k : (Eager L mem).KeyedStream p c α ord ret) :
    ((Eager L mem).values k).den = (Values L mem).values k.den := rfl

theorem eag_weaken_retries_den {α : Type} [DecidableEq α] {ord : StrOrd}
    (s : (Eager L mem).Stream ℓ α ord .exactlyOnce) :
    ((Eager L mem).weaken_retries s).den
      = (Values L mem).weaken_retries s.den := rfl

theorem eag_union_den {α : Type} [DecidableEq α] {ret : Retries}
    (a b : (Eager L mem).Stream ℓ α .noOrder ret) :
    ((Eager L mem).union a b).den
      = (Values L mem).union a.den b.den := rfl

theorem eag_assume_ordering_den {α : Type} [DecidableEq α]
    (u : (Eager L mem).Stream ℓ α .noOrder .exactlyOnce)
    (d : (Values L mem).OrderSelDec (mem ℓ) α) :
    ((Eager L mem).assume_ordering u d).den
      = (Values L mem).assume_ordering u.den d := rfl

theorem eag_fold_den {α σ : Type} [DecidableEq α] {ord : StrOrd}
    {ret : Retries} (g : σ → α → σ) (init : σ) (ok : FoldOk ord ret g)
    (s : (Eager L mem).Stream ℓ α ord ret) :
    ((Eager L mem).fold (ℓ := ℓ) g init ok s).den
      = (Values L mem).fold g init ok s.den := rfl

theorem eag_fold_monotone_den {α σ : Type} [DecidableEq α]
    {ord : StrOrd} {ret : Retries} (vo : ValueOrder σ) (g : σ → α → σ)
    (init : σ) (ok : FoldOk ord ret g) (hinfl : ∀ s x, vo.le s (g s x))
    (s : (Eager L mem).Stream ℓ α ord ret) :
    ((Eager L mem).fold_monotone (ℓ := ℓ) vo g init ok hinfl s).den
      = (Values L mem).fold_monotone vo g init ok hinfl s.den := rfl

theorem eag_snapshot_unbounded_den {α σ : Type} [DecidableEq α]
    {ord : StrOrd} {ret : Retries}
    (s : (Eager L mem).Singleton ℓ α σ ord ret .unbounded)
    (d : (Values L mem).SnapDec (mem ℓ) α ord) :
    ((Eager L mem).snapshot (ret := ret) (b := .unbounded) s d).den
      = (Values L mem).snapshot (ret := ret) (b := .unbounded) s.den d
    := rfl

theorem eag_snapshot_monotonic_den {α σ : Type} [DecidableEq α]
    {ord : StrOrd} {ret : Retries} {vo : ValueOrder σ}
    (s : (Eager L mem).Singleton ℓ α σ ord ret (.monotonic vo))
    (d : (Values L mem).SnapDec (mem ℓ) α ord) :
    ((Eager L mem).snapshot (ret := ret) (b := .monotonic vo) s d).den
      = (Values L mem).snapshot (ret := ret) (b := .monotonic vo) s.den d
    := rfl

theorem eag_batch_den {α : Type} [DecidableEq α]
    (s : (Eager L mem).Stream ℓ α .noOrder .exactlyOnce)
    (d : (Values L mem).BatchDec (mem ℓ) α) :
    ((Eager L mem).batch s d).den = (Values L mem).batch s.den d := rfl

theorem eag_batch_ordered_den {α : Type} [DecidableEq α]
    (s : (Eager L mem).Stream ℓ α .totalOrder .exactlyOnce)
    (d : (Values L mem).OrdBatchDec (mem ℓ)) :
    ((Eager L mem).batch_ordered s d).den
      = (Values L mem).batch_ordered s.den d := rfl

theorem eag_assume_ordering_batch_den {α : Type} [DecidableEq α]
    (bs : (Eager L mem).TickStream ℓ α .noOrder .exactlyOnce)
    (d : (Values L mem).BatchOrdSelDec (mem ℓ) α) :
    ((Eager L mem).assume_ordering_batch bs d).den
      = (Values L mem).assume_ordering_batch bs.den d := rfl

theorem eag_mapBatchWith_den {α σ β : Type} [DecidableEq α]
    (bs : (Eager L mem).TickStream ℓ α .totalOrder .exactlyOnce)
    (t : (Eager L mem).TickSingleton ℓ σ .unbounded)
    (f : Fin (mem ℓ) → List α → σ → β) :
    ((Eager L mem).mapBatchWith bs t f).den
      = (Values L mem).mapBatchWith bs.den t.den f := rfl

theorem eag_mapBatch_den {α β : Type} [DecidableEq α]
    (bs : (Eager L mem).TickStream ℓ α .totalOrder .exactlyOnce)
    (f : Fin (mem ℓ) → List α → β) :
    ((Eager L mem).mapBatch bs f).den
      = (Values L mem).mapBatch bs.den f := rfl

theorem eag_mapBatchesWith_den {α σ β : Type} [DecidableEq α]
    [DecidableEq β]
    (bs : (Eager L mem).TickStream ℓ α .noOrder .exactlyOnce)
    (t : (Eager L mem).TickSingleton ℓ σ .unbounded)
    (f : Fin (mem ℓ) → α → σ → β) :
    ((Eager L mem).mapBatchesWith bs t f).den
      = (Values L mem).mapBatchesWith bs.den t.den f := rfl

theorem eag_scan_batches_across_ticks_den {α τ σ β : Type}
    [DecidableEq α]
    (bs : (Eager L mem).TickStream ℓ α .totalOrder .exactlyOnce)
    (t : (Eager L mem).TickSingleton ℓ τ .unbounded)
    (g : Fin (mem ℓ) → σ → List α → τ → σ × β) (init : σ) :
    ((Eager L mem).scan_batches_across_ticks bs t g init).den
      = (Values L mem).scan_batches_across_ticks bs.den t.den g init
    := rfl

theorem eag_fold_batches_across_ticks_monotone_den {α σ : Type}
    [DecidableEq α] (vo : ValueOrder σ)
    (g : Fin (mem ℓ) → σ → α → σ) (init : σ)
    (comm : ∀ i s x y, g i (g i s x) y = g i (g i s y) x)
    (hinfl : ∀ i s x, vo.le s (g i s x))
    (bs : (Eager L mem).TickStream ℓ α .noOrder .exactlyOnce) :
    ((Eager L mem).fold_batches_across_ticks_monotone
        vo g init comm hinfl bs).den
      = (Values L mem).fold_batches_across_ticks_monotone
          vo g init comm hinfl bs.den := rfl

theorem eag_sample_every_den {α : Type} [DecidableEq α]
    (t : (Eager L mem).TickSingleton ℓ (Option α) .unbounded)
    (d : (Values L mem).SampleDec (mem ℓ)) :
    ((Eager L mem).sample_every t d).den
      = (Values L mem).sample_every t.den d := rfl

theorem eag_timeout_snapshot_den {α : Type} [DecidableEq α]
    {ord : StrOrd} {ret : Retries}
    (s : (Eager L mem).Stream ℓ α ord ret)
    (d : (Values L mem).TimerDec (mem ℓ)) :
    ((Eager L mem).timeout_snapshot s d).den
      = (Values L mem).timeout_snapshot s.den d := rfl

theorem eag_source_interval_batch_den
    (d : (Values L mem).PulseDec (mem ℓ)) :
    ((Eager L mem).source_interval_batch (ℓ := ℓ) d).den
      = (Values L mem).source_interval_batch (ℓ := ℓ) d := rfl

theorem eag_scan_batches_unordered_across_ticks_den {α τ σ β : Type}
    [DecidableEq α]
    (bs : (Eager L mem).TickStream ℓ α .noOrder .exactlyOnce)
    (t : (Eager L mem).TickSingleton ℓ τ .unbounded)
    (g : Fin (mem ℓ) → σ → Multiset α → τ → σ × β) (init : σ) :
    ((Eager L mem).scan_batches_unordered_across_ticks bs t g init).den
      = (Values L mem).scan_batches_unordered_across_ticks
          bs.den t.den g init := rfl

theorem eag_scan_batches_unordered_den {α σ β : Type} [DecidableEq α]
    (bs : (Eager L mem).TickStream ℓ α .noOrder .exactlyOnce)
    (g : Fin (mem ℓ) → σ → Multiset α → σ × β) (init : σ) :
    ((Eager L mem).scan_batches_unordered bs g init).den
      = (Values L mem).scan_batches_unordered bs.den g init := rfl

theorem eag_scan_batches_unordered₂_den {α γ σ β : Type}
    [DecidableEq α] [DecidableEq γ]
    (bs : (Eager L mem).TickStream ℓ α .noOrder .exactlyOnce)
    (cs : (Eager L mem).TickStream ℓ γ .noOrder .exactlyOnce)
    (g : Fin (mem ℓ) → σ → Multiset α → Multiset γ → σ × β) (init : σ) :
    ((Eager L mem).scan_batches_unordered₂ bs cs g init).den
      = (Values L mem).scan_batches_unordered₂ bs.den cs.den g init
    := rfl

theorem eag_scan_across_ticks_den {α σ β : Type}
    (t : (Eager L mem).TickSingleton ℓ α .unbounded)
    (g : Fin (mem ℓ) → σ → α → σ × β) (init : σ) :
    ((Eager L mem).scan_across_ticks t g init).den
      = (Values L mem).scan_across_ticks t.den g init := rfl

theorem eag_mapTick_den {α β : Type}
    (s : (Eager L mem).TickSingleton ℓ α .unbounded)
    (f : Fin (mem ℓ) → α → β) :
    ((Eager L mem).mapTick s f).den
      = (Values L mem).mapTick s.den f := rfl

theorem eag_zipTick_den {α β : Type}
    (a : (Eager L mem).TickSingleton ℓ α .unbounded)
    (b : (Eager L mem).TickSingleton ℓ β .unbounded) :
    ((Eager L mem).zipTick a b).den
      = (Values L mem).zipTick a.den b.den := rfl

theorem eag_fold_across_ticks_monotone_den {α σ : Type}
    (vo : ValueOrder σ) (g : Fin (mem ℓ) → σ → α → σ) (init : σ)
    (hinfl : ∀ i s x, vo.le s (g i s x))
    (s : (Eager L mem).TickSingleton ℓ α .unbounded) :
    ((Eager L mem).fold_across_ticks_monotone vo g init hinfl s).den
      = (Values L mem).fold_across_ticks_monotone vo g init hinfl s.den
    := rfl

theorem eag_mapMonotone_den {σ τ : Type} {vo : ValueOrder σ}
    (vo' : ValueOrder τ)
    (m : (Eager L mem).TickSingleton ℓ σ (.monotonic vo))
    (h : Fin (mem ℓ) → σ → τ)
    (hpres : ∀ i {a b}, vo.le a b → vo'.le (h i a) (h i b)) :
    ((Eager L mem).mapMonotone vo' m h hpres).den
      = (Values L mem).mapMonotone vo' m.den h hpres := rfl

theorem eag_forgetBound_den {σ : Type} {vo : ValueOrder σ}
    (m : (Eager L mem).TickSingleton ℓ σ (.monotonic vo)) :
    ((Eager L mem).forgetBound m).den
      = (Values L mem).forgetBound m.den := rfl

theorem eag_defer_den {σ : Type} (init : σ)
    (t : (Eager L mem).TickSingleton ℓ σ .unbounded) :
    ((Eager L mem).defer init t).den
      = (Values L mem).defer init t.den := rfl

theorem eag_filterMapBatchesWith_den {α σ β : Type} [DecidableEq α]
    [DecidableEq β]
    (bs : (Eager L mem).TickStream ℓ α .noOrder .exactlyOnce)
    (t : (Eager L mem).TickSingleton ℓ σ .unbounded)
    (f : Fin (mem ℓ) → α → σ → Option β) :
    ((Eager L mem).filterMapBatchesWith bs t f).den
      = (Values L mem).filterMapBatchesWith bs.den t.den f := rfl

theorem eag_mapBatchesUnordered_den {α σ β : Type} [DecidableEq α]
    (bs : (Eager L mem).TickStream ℓ α .noOrder .exactlyOnce)
    (t : (Eager L mem).TickSingleton ℓ σ .unbounded)
    (f : Fin (mem ℓ) → Multiset α → σ → β) :
    ((Eager L mem).mapBatchesUnordered bs t f).den
      = (Values L mem).mapBatchesUnordered bs.den t.den f := rfl

theorem eag_emitMultisetBatches_den {β : Type} [DecidableEq β]
    (t : (Eager L mem).TickSingleton ℓ (Multiset β) .unbounded)
    (d : (Values L mem).EmitDec (mem ℓ) β) :
    ((Eager L mem).emitMultisetBatches t d).den
      = (Values L mem).emitMultisetBatches t.den d := rfl

theorem eag_allTicks_den {β : Type} [DecidableEq β] {ord : StrOrd}
    (bs : (Eager L mem).TickStream ℓ β ord .exactlyOnce) :
    ((Eager L mem).allTicks bs).den
      = (Values L mem).allTicks bs.den := rfl

theorem eag_emitBatches_den {β : Type} [DecidableEq β]
    (t : (Eager L mem).TickSingleton ℓ (List β) .unbounded) :
    ((Eager L mem).emitBatches t).den
      = (Values L mem).emitBatches t.den := rfl

theorem eag_emitBatchesUnordered_den {β : Type} [DecidableEq β]
    (t : (Eager L mem).TickSingleton ℓ (List β) .unbounded) :
    ((Eager L mem).emitBatchesUnordered t).den
      = (Values L mem).emitBatchesUnordered t.den := rfl

theorem eag_fix_stream_den {α : Type} [DecidableEq α] {ord : StrOrd}
    {ret : Retries} (fuel : (Values L mem).FixDec)
    (body : (Eager L mem).Stream ℓ α ord ret →
      (Eager L mem).Stream ℓ α ord ret) :
    ((Eager L mem).fix_stream (ℓ := ℓ) fuel body).den
      = (Values L mem).fix_stream (ℓ := ℓ) (α := α) (ord := ord)
          (ret := ret) fuel
          (fun v => (body (EagStream.embedV v)).den) := rfl

theorem eag_fix_tick_den {σ : Type} (fuel : (Values L mem).FixDec)
    (body : (Eager L mem).TickSingleton ℓ σ .unbounded →
      (Eager L mem).TickSingleton ℓ σ .unbounded) :
    ((Eager L mem).fix_tick (ℓ := ℓ) fuel body).den
      = (Values L mem).fix_tick (ℓ := ℓ) (σ := σ) fuel
          (fun v => (body (EagTickSing.embedV v)).den) := rfl

/-- The eager projection simp set: pushes `.den` through every op
(no terminal closer — compose freely in structural proofs). -/
macro "eag_simp" "[" ids:Lean.Parser.Tactic.simpLemma,* "]" : tactic =>
  `(tactic| (
    simp only [$ids,*,
      eag_stream_embedV_den, eag_ticksing_embedV_den,
      eag_stream_input_den, eag_ticksing_input_den,
      eag_map_den, eag_filterMap_den, eag_broadcast_den, eag_demux_den,
      eag_values_den, eag_weaken_retries_den, eag_union_den,
      eag_assume_ordering_den, eag_fold_den, eag_fold_monotone_den,
      eag_snapshot_unbounded_den, eag_snapshot_monotonic_den,
      eag_batch_den, eag_batch_ordered_den,
      eag_assume_ordering_batch_den,
      eag_mapBatchWith_den, eag_mapBatch_den, eag_mapBatchesWith_den,
      eag_scan_batches_across_ticks_den,
      eag_fold_batches_across_ticks_monotone_den,
      eag_sample_every_den, eag_timeout_snapshot_den,
      eag_source_interval_batch_den,
      eag_scan_batches_unordered_across_ticks_den,
      eag_scan_batches_unordered_den, eag_scan_batches_unordered₂_den,
      eag_scan_across_ticks_den, eag_mapTick_den, eag_zipTick_den,
      eag_fold_across_ticks_monotone_den, eag_mapMonotone_den,
      eag_forgetBound_den, eag_defer_den,
      eag_filterMapBatchesWith_den, eag_mapBatchesUnordered_den,
      eag_emitMultisetBatches_den, eag_allTicks_den,
      eag_emitBatches_den, eag_emitBatchesUnordered_den,
      eag_fix_stream_den, eag_fix_tick_den]))

/-- `eager_transfer [defs…]`: unfold the listed program definitions and
push the eager projection through — the whole projection identity of a
program, from the generic per-op lemmas. The trailing `rfl` collapses
the input leaves (leaf-local defeq only). -/
macro "eager_transfer" "[" ids:Lean.Parser.Tactic.simpLemma,* "]" : tactic =>
  `(tactic| (eag_simp [$ids,*]; with_reducible rfl))

end HydroV2
