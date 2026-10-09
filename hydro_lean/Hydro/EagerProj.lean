import Hydro.Eager

/-!
# Hydro · generic eager-projection lemmas (`EagerProj`)

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

namespace Hydro

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
    (v : (Values L mem).Ticked ℓ σ) :
    (Eager L mem).Ticked ℓ σ := EPack.ofDen v

/-- A materialized stream input (executables). -/
def EagStream.input {α : Type} [DecidableEq α] {ord : StrOrd}
    {ret : Retries} (d : Vector (PoolCarrier α ord ret) (mem ℓ)) :
    (Eager L mem).Stream ℓ α ord ret := EPack.ofData d

/-- A materialized tick-singleton input (executables). -/
def EagTickSing.input {σ : Type} (d : Vector (Trace σ) (mem ℓ)) :
    (Eager L mem).Ticked ℓ σ := EPack.ofData d

/-- A materialized async-singleton input (executables): per-member
read functions over source cuts (B2's checkpoint shape). -/
def EagSing.input {α σ : Type} [DecidableEq α] {ord : StrOrd}
    {ret : Retries} (d : Vector (CutDec α ord → Trace σ) (mem ℓ)) :
    (Eager L mem).Singleton ℓ α σ ord ret .unbounded := EPack.ofData d

@[simp] theorem eag_stream_embedV_den {α : Type} [DecidableEq α]
    {ord : StrOrd} {ret : Retries}
    (v : (Values L mem).Stream ℓ α ord ret) :
    (EagStream.embedV (ℓ := ℓ) v).den = v := rfl

@[simp] theorem eag_ticksing_embedV_den {σ : Type}
    (v : (Values L mem).Ticked ℓ σ) :
    (EagTickSing.embedV (ℓ := ℓ) v).den = v := rfl

@[simp] theorem eag_stream_input_den {α : Type} [DecidableEq α]
    {ord : StrOrd} {ret : Retries}
    (d : Vector (PoolCarrier α ord ret) (mem ℓ)) :
    (EagStream.input (ℓ := ℓ) d).den = fun i => d.get i := rfl

@[simp] theorem eag_ticksing_input_den {σ : Type}
    (d : Vector (Trace σ) (mem ℓ)) :
    (EagTickSing.input (ℓ := ℓ) d).den = fun i => d.get i := rfl

@[simp] theorem eag_sing_input_den {α σ : Type} [DecidableEq α]
    {ord : StrOrd} {ret : Retries}
    (d : Vector (CutDec α ord → Trace σ) (mem ℓ)) :
    (EagSing.input (ℓ := ℓ) (ret := ret) d).den = fun i => d.get i :=
  rfl

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

theorem eag_broadcast_closed_den {c p : L} {α : Type} [DecidableEq α]
    {ord : StrOrd} {ret : Retries}
    (d : (Values L mem).TransportDec (mem p) (mem c))
    (s : (Eager L mem).Stream c α ord ret) :
    ((Eager L mem).broadcast_closed (p := p) d s).den
      = (Values L mem).broadcast_closed d s.den := rfl

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

theorem eag_sample_every_den {α : Type} [DecidableEq α]
    (t : (Eager L mem).Ticked ℓ (Option α))
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

theorem eag_mapTick_den {α β : Type}
    (s : (Eager L mem).Ticked ℓ α)
    (f : Fin (mem ℓ) → α → β) :
    ((Eager L mem).mapTick s f).den
      = (Values L mem).mapTick s.den f := rfl

theorem eag_zipTick_den {α β : Type}
    (a : (Eager L mem).Ticked ℓ α)
    (b : (Eager L mem).Ticked ℓ β) :
    ((Eager L mem).zipTick a b).den
      = (Values L mem).zipTick a.den b.den := rfl

/-! The shaped tick former at `Eager`: `.den` of each output-path
projection is the `Values` former's same projection on the `.den`
legs (path rules, as at the corner — no `pack` intermediate). -/
section EagTickScanPaths
variable {a b c : TickShape} {τ : Type} {inst : DecidableEq α} {ord : StrOrd} {ret : Retries}
  (sts ins : TickShape)
theorem eag_tick_scan_0_sing_den (x : TickedOf (Eager L mem).Ticked (Eager L mem).TickStream ℓ (ins)) (g : Fin (mem ℓ) → BoundedOf (Eager L mem).BoundedSingleton (Eager L mem).BoundedStream (sts) → BoundedOf (Eager L mem).BoundedSingleton (Eager L mem).BoundedStream (ins) → BoundedOf (Eager L mem).BoundedSingleton (Eager L mem).BoundedStream (sts) × BoundedOf (Eager L mem).BoundedSingleton (Eager L mem).BoundedStream (.sing τ)) (init : SeedOf sts) :
    ((Eager L mem).tick_scan (ℓ := ℓ) sts ins (.sing τ) x g init).den = (Values L mem).tick_scan sts ins (.sing τ) (EagerTick.den ins x) g init := rfl
theorem eag_tick_scan_0_stream_den (x : TickedOf (Eager L mem).Ticked (Eager L mem).TickStream ℓ (ins)) (g : Fin (mem ℓ) → BoundedOf (Eager L mem).BoundedSingleton (Eager L mem).BoundedStream (sts) → BoundedOf (Eager L mem).BoundedSingleton (Eager L mem).BoundedStream (ins) → BoundedOf (Eager L mem).BoundedSingleton (Eager L mem).BoundedStream (sts) × BoundedOf (Eager L mem).BoundedSingleton (Eager L mem).BoundedStream (.stream α inst ord ret)) (init : SeedOf sts) :
    ((Eager L mem).tick_scan (ℓ := ℓ) sts ins (.stream α inst ord ret) x g init).den = (Values L mem).tick_scan sts ins (.stream α inst ord ret) (EagerTick.den ins x) g init := rfl
theorem eag_tick_scan_1_sing_den (x : TickedOf (Eager L mem).Ticked (Eager L mem).TickStream ℓ (ins)) (g : Fin (mem ℓ) → BoundedOf (Eager L mem).BoundedSingleton (Eager L mem).BoundedStream (sts) → BoundedOf (Eager L mem).BoundedSingleton (Eager L mem).BoundedStream (ins) → BoundedOf (Eager L mem).BoundedSingleton (Eager L mem).BoundedStream (sts) × BoundedOf (Eager L mem).BoundedSingleton (Eager L mem).BoundedStream (.pair (.sing τ) a)) (init : SeedOf sts) :
    (((Eager L mem).tick_scan (ℓ := ℓ) sts ins (.pair (.sing τ) a) x g init).1).den = ((Values L mem).tick_scan sts ins (.pair (.sing τ) a) (EagerTick.den ins x) g init).1 := rfl
theorem eag_tick_scan_1_stream_den (x : TickedOf (Eager L mem).Ticked (Eager L mem).TickStream ℓ (ins)) (g : Fin (mem ℓ) → BoundedOf (Eager L mem).BoundedSingleton (Eager L mem).BoundedStream (sts) → BoundedOf (Eager L mem).BoundedSingleton (Eager L mem).BoundedStream (ins) → BoundedOf (Eager L mem).BoundedSingleton (Eager L mem).BoundedStream (sts) × BoundedOf (Eager L mem).BoundedSingleton (Eager L mem).BoundedStream (.pair (.stream α inst ord ret) a)) (init : SeedOf sts) :
    (((Eager L mem).tick_scan (ℓ := ℓ) sts ins (.pair (.stream α inst ord ret) a) x g init).1).den = ((Values L mem).tick_scan sts ins (.pair (.stream α inst ord ret) a) (EagerTick.den ins x) g init).1 := rfl
theorem eag_tick_scan_2_sing_den (x : TickedOf (Eager L mem).Ticked (Eager L mem).TickStream ℓ (ins)) (g : Fin (mem ℓ) → BoundedOf (Eager L mem).BoundedSingleton (Eager L mem).BoundedStream (sts) → BoundedOf (Eager L mem).BoundedSingleton (Eager L mem).BoundedStream (ins) → BoundedOf (Eager L mem).BoundedSingleton (Eager L mem).BoundedStream (sts) × BoundedOf (Eager L mem).BoundedSingleton (Eager L mem).BoundedStream (.pair a (.sing τ))) (init : SeedOf sts) :
    (((Eager L mem).tick_scan (ℓ := ℓ) sts ins (.pair a (.sing τ)) x g init).2).den = ((Values L mem).tick_scan sts ins (.pair a (.sing τ)) (EagerTick.den ins x) g init).2 := rfl
theorem eag_tick_scan_2_stream_den (x : TickedOf (Eager L mem).Ticked (Eager L mem).TickStream ℓ (ins)) (g : Fin (mem ℓ) → BoundedOf (Eager L mem).BoundedSingleton (Eager L mem).BoundedStream (sts) → BoundedOf (Eager L mem).BoundedSingleton (Eager L mem).BoundedStream (ins) → BoundedOf (Eager L mem).BoundedSingleton (Eager L mem).BoundedStream (sts) × BoundedOf (Eager L mem).BoundedSingleton (Eager L mem).BoundedStream (.pair a (.stream α inst ord ret))) (init : SeedOf sts) :
    (((Eager L mem).tick_scan (ℓ := ℓ) sts ins (.pair a (.stream α inst ord ret)) x g init).2).den = ((Values L mem).tick_scan sts ins (.pair a (.stream α inst ord ret)) (EagerTick.den ins x) g init).2 := rfl
theorem eag_tick_scan_21_sing_den (x : TickedOf (Eager L mem).Ticked (Eager L mem).TickStream ℓ (ins)) (g : Fin (mem ℓ) → BoundedOf (Eager L mem).BoundedSingleton (Eager L mem).BoundedStream (sts) → BoundedOf (Eager L mem).BoundedSingleton (Eager L mem).BoundedStream (ins) → BoundedOf (Eager L mem).BoundedSingleton (Eager L mem).BoundedStream (sts) × BoundedOf (Eager L mem).BoundedSingleton (Eager L mem).BoundedStream (.pair a (.pair (.sing τ) b))) (init : SeedOf sts) :
    ((((Eager L mem).tick_scan (ℓ := ℓ) sts ins (.pair a (.pair (.sing τ) b)) x g init).2).1).den = (((Values L mem).tick_scan sts ins (.pair a (.pair (.sing τ) b)) (EagerTick.den ins x) g init).2).1 := rfl
theorem eag_tick_scan_21_stream_den (x : TickedOf (Eager L mem).Ticked (Eager L mem).TickStream ℓ (ins)) (g : Fin (mem ℓ) → BoundedOf (Eager L mem).BoundedSingleton (Eager L mem).BoundedStream (sts) → BoundedOf (Eager L mem).BoundedSingleton (Eager L mem).BoundedStream (ins) → BoundedOf (Eager L mem).BoundedSingleton (Eager L mem).BoundedStream (sts) × BoundedOf (Eager L mem).BoundedSingleton (Eager L mem).BoundedStream (.pair a (.pair (.stream α inst ord ret) b))) (init : SeedOf sts) :
    ((((Eager L mem).tick_scan (ℓ := ℓ) sts ins (.pair a (.pair (.stream α inst ord ret) b)) x g init).2).1).den = (((Values L mem).tick_scan sts ins (.pair a (.pair (.stream α inst ord ret) b)) (EagerTick.den ins x) g init).2).1 := rfl
theorem eag_tick_scan_22_sing_den (x : TickedOf (Eager L mem).Ticked (Eager L mem).TickStream ℓ (ins)) (g : Fin (mem ℓ) → BoundedOf (Eager L mem).BoundedSingleton (Eager L mem).BoundedStream (sts) → BoundedOf (Eager L mem).BoundedSingleton (Eager L mem).BoundedStream (ins) → BoundedOf (Eager L mem).BoundedSingleton (Eager L mem).BoundedStream (sts) × BoundedOf (Eager L mem).BoundedSingleton (Eager L mem).BoundedStream (.pair a (.pair b (.sing τ)))) (init : SeedOf sts) :
    ((((Eager L mem).tick_scan (ℓ := ℓ) sts ins (.pair a (.pair b (.sing τ))) x g init).2).2).den = (((Values L mem).tick_scan sts ins (.pair a (.pair b (.sing τ))) (EagerTick.den ins x) g init).2).2 := rfl
theorem eag_tick_scan_22_stream_den (x : TickedOf (Eager L mem).Ticked (Eager L mem).TickStream ℓ (ins)) (g : Fin (mem ℓ) → BoundedOf (Eager L mem).BoundedSingleton (Eager L mem).BoundedStream (sts) → BoundedOf (Eager L mem).BoundedSingleton (Eager L mem).BoundedStream (ins) → BoundedOf (Eager L mem).BoundedSingleton (Eager L mem).BoundedStream (sts) × BoundedOf (Eager L mem).BoundedSingleton (Eager L mem).BoundedStream (.pair a (.pair b (.stream α inst ord ret)))) (init : SeedOf sts) :
    ((((Eager L mem).tick_scan (ℓ := ℓ) sts ins (.pair a (.pair b (.stream α inst ord ret))) x g init).2).2).den = (((Values L mem).tick_scan sts ins (.pair a (.pair b (.stream α inst ord ret))) (EagerTick.den ins x) g init).2).2 := rfl
theorem eag_tick_scan_221_sing_den (x : TickedOf (Eager L mem).Ticked (Eager L mem).TickStream ℓ (ins)) (g : Fin (mem ℓ) → BoundedOf (Eager L mem).BoundedSingleton (Eager L mem).BoundedStream (sts) → BoundedOf (Eager L mem).BoundedSingleton (Eager L mem).BoundedStream (ins) → BoundedOf (Eager L mem).BoundedSingleton (Eager L mem).BoundedStream (sts) × BoundedOf (Eager L mem).BoundedSingleton (Eager L mem).BoundedStream (.pair a (.pair b (.pair (.sing τ) c)))) (init : SeedOf sts) :
    (((((Eager L mem).tick_scan (ℓ := ℓ) sts ins (.pair a (.pair b (.pair (.sing τ) c))) x g init).2).2).1).den = ((((Values L mem).tick_scan sts ins (.pair a (.pair b (.pair (.sing τ) c))) (EagerTick.den ins x) g init).2).2).1 := rfl
theorem eag_tick_scan_221_stream_den (x : TickedOf (Eager L mem).Ticked (Eager L mem).TickStream ℓ (ins)) (g : Fin (mem ℓ) → BoundedOf (Eager L mem).BoundedSingleton (Eager L mem).BoundedStream (sts) → BoundedOf (Eager L mem).BoundedSingleton (Eager L mem).BoundedStream (ins) → BoundedOf (Eager L mem).BoundedSingleton (Eager L mem).BoundedStream (sts) × BoundedOf (Eager L mem).BoundedSingleton (Eager L mem).BoundedStream (.pair a (.pair b (.pair (.stream α inst ord ret) c)))) (init : SeedOf sts) :
    (((((Eager L mem).tick_scan (ℓ := ℓ) sts ins (.pair a (.pair b (.pair (.stream α inst ord ret) c))) x g init).2).2).1).den = ((((Values L mem).tick_scan sts ins (.pair a (.pair b (.pair (.stream α inst ord ret) c))) (EagerTick.den ins x) g init).2).2).1 := rfl
theorem eag_tick_scan_222_sing_den (x : TickedOf (Eager L mem).Ticked (Eager L mem).TickStream ℓ (ins)) (g : Fin (mem ℓ) → BoundedOf (Eager L mem).BoundedSingleton (Eager L mem).BoundedStream (sts) → BoundedOf (Eager L mem).BoundedSingleton (Eager L mem).BoundedStream (ins) → BoundedOf (Eager L mem).BoundedSingleton (Eager L mem).BoundedStream (sts) × BoundedOf (Eager L mem).BoundedSingleton (Eager L mem).BoundedStream (.pair a (.pair b (.pair c (.sing τ))))) (init : SeedOf sts) :
    (((((Eager L mem).tick_scan (ℓ := ℓ) sts ins (.pair a (.pair b (.pair c (.sing τ)))) x g init).2).2).2).den = ((((Values L mem).tick_scan sts ins (.pair a (.pair b (.pair c (.sing τ)))) (EagerTick.den ins x) g init).2).2).2 := rfl
theorem eag_tick_scan_222_stream_den (x : TickedOf (Eager L mem).Ticked (Eager L mem).TickStream ℓ (ins)) (g : Fin (mem ℓ) → BoundedOf (Eager L mem).BoundedSingleton (Eager L mem).BoundedStream (sts) → BoundedOf (Eager L mem).BoundedSingleton (Eager L mem).BoundedStream (ins) → BoundedOf (Eager L mem).BoundedSingleton (Eager L mem).BoundedStream (sts) × BoundedOf (Eager L mem).BoundedSingleton (Eager L mem).BoundedStream (.pair a (.pair b (.pair c (.stream α inst ord ret))))) (init : SeedOf sts) :
    (((((Eager L mem).tick_scan (ℓ := ℓ) sts ins (.pair a (.pair b (.pair c (.stream α inst ord ret)))) x g init).2).2).2).den = ((((Values L mem).tick_scan sts ins (.pair a (.pair b (.pair c (.stream α inst ord ret)))) (EagerTick.den ins x) g init).2).2).2 := rfl
end EagTickScanPaths

/-! The `EagerTick` structural rules, at the interpretations' own carrier
types (what the simp unifier sees in a program's goals). -/
section EagerTickRules
variable {a b : TickShape} {τ : Type} {α : Type} {inst : DecidableEq α}
  {ord : StrOrd} {ret : Retries}
theorem eag_den_pair (x : TickedOf (Eager L mem).Ticked (Eager L mem).TickStream ℓ (.pair a b)) :
    (EagerTick.den (.pair a b) x : TickedOf (Values L mem).Ticked (Values L mem).TickStream ℓ (.pair a b))
      = (EagerTick.den a x.1, EagerTick.den b x.2) := rfl
theorem eag_den_sing (x : (Eager L mem).Ticked ℓ τ) :
    (EagerTick.den (.sing τ) x : (Values L mem).Ticked ℓ τ) = x.den := rfl
theorem eag_den_stream (x : (Eager L mem).TickStream ℓ α ord ret) :
    (EagerTick.den (.stream α inst ord ret) x : (Values L mem).TickStream ℓ α ord ret) = x.den := rfl
end EagerTickRules

theorem eag_defer_tick_den {σ : Type} (init : σ)
    (t : (Eager L mem).Ticked ℓ σ) :
    ((Eager L mem).defer_tick init t).den
      = (Values L mem).defer_tick init t.den := rfl

theorem eag_allTicks_den {β : Type} [DecidableEq β] {ord : StrOrd}
    (bs : (Eager L mem).TickStream ℓ β ord .exactlyOnce) :
    ((Eager L mem).allTicks bs).den
      = (Values L mem).allTicks bs.den := rfl

theorem eag_flattenOrdered_den {β : Type} [DecidableEq β]
    (t : (Eager L mem).Ticked ℓ (List β)) :
    ((Eager L mem).flattenOrdered t).den
      = (Values L mem).flattenOrdered t.den := rfl

theorem eag_flattenUnordered_den {β : Type} [DecidableEq β]
    (t : (Eager L mem).Ticked ℓ (List β)) :
    ((Eager L mem).flattenUnordered t).den
      = (Values L mem).flattenUnordered t.den := rfl

theorem eag_fix_stream_den {α : Type} [DecidableEq α] {ord : StrOrd}
    {ret : Retries} (fuel : (Values L mem).FixDec)
    (body : (Eager L mem).Stream ℓ α ord ret →
      (Eager L mem).Stream ℓ α ord ret) :
    ((Eager L mem).fix_stream (ℓ := ℓ) fuel body).den
      = (Values L mem).fix_stream (ℓ := ℓ) (α := α) (ord := ord)
          (ret := ret) fuel
          (fun v => (body (EagStream.embedV v)).den) := rfl

theorem eag_fix_tick_den {σ : Type} (fuel : (Values L mem).FixDec)
    (body : (Eager L mem).Ticked ℓ σ →
      (Eager L mem).Ticked ℓ σ) :
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
      eag_map_den, eag_filterMap_den, eag_broadcast_closed_den, eag_demux_den,
      eag_values_den, eag_weaken_retries_den, eag_union_den,
      eag_assume_ordering_den, eag_fold_den, eag_fold_monotone_den,
      eag_snapshot_unbounded_den, eag_snapshot_monotonic_den,
      eag_batch_den, eag_batch_ordered_den,
      eag_sample_every_den, eag_timeout_snapshot_den,
      eag_source_interval_batch_den,
      eag_mapTick_den, eag_zipTick_den,
      eag_defer_tick_den,
      eag_tick_scan_0_sing_den, eag_tick_scan_0_stream_den, eag_tick_scan_1_sing_den, eag_tick_scan_1_stream_den, eag_tick_scan_2_sing_den, eag_tick_scan_2_stream_den, eag_tick_scan_21_sing_den, eag_tick_scan_21_stream_den, eag_tick_scan_22_sing_den, eag_tick_scan_22_stream_den, eag_tick_scan_221_sing_den, eag_tick_scan_221_stream_den, eag_tick_scan_222_sing_den, eag_tick_scan_222_stream_den,
      eag_den_pair, eag_den_sing, eag_den_stream,
      eag_allTicks_den,
      eag_flattenOrdered_den, eag_flattenUnordered_den,
      eag_fix_stream_den, eag_fix_tick_den]))

/-- `eager_transfer [defs…]`: unfold the listed program definitions and
push the eager projection through — the whole projection identity of a
program, from the generic per-op lemmas. The trailing `rfl` collapses
the input leaves (leaf-local defeq only). -/
macro "eager_transfer" "[" ids:Lean.Parser.Tactic.simpLemma,* "]" : tactic =>
  `(tactic| (eag_simp [$ids,*]; with_reducible rfl))

end Hydro
