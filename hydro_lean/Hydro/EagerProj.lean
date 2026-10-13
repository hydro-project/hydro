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
    (v : (Values L mem).Atomic ℓ σ) :
    (Eager L mem).Atomic ℓ σ := EPack.ofDen v

/-- A materialized stream input (executables). -/
def EagStream.input {α : Type} [DecidableEq α] {ord : StrOrd}
    {ret : Retries} (d : Vector (PoolCarrier α ord ret) (mem ℓ)) :
    (Eager L mem).Stream ℓ α ord ret := EPack.ofData d

/-- A materialized tick-singleton input (executables). -/
def EagTickSing.input {σ : Type} (d : Vector (Trace σ) (mem ℓ)) :
    (Eager L mem).Atomic ℓ σ := EPack.ofData d

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
    (v : (Values L mem).Atomic ℓ σ) :
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

theorem eag_values_den {K V : Type} [DecidableEq K] [DecidableEq V]
    {ord : StrOrd} {ret : Retries}
    (k : (Eager L mem).KeyedStream ℓ K V ord ret) :
    ((Eager L mem).values k).den = (Values L mem).values k.den := rfl

theorem eag_into_keyed_den {K V : Type} [DecidableEq K] [DecidableEq V] {ord : StrOrd}
    (s : (Eager L mem).Stream ℓ (K × V) ord .exactlyOnce) :
    ((Eager L mem).into_keyed s).den = (Values L mem).into_keyed s.den := rfl

theorem eag_kassume_ordering_den {K V : Type} [DecidableEq K] [DecidableEq V]
    (u : (Eager L mem).KeyedStream ℓ K V .noOrder .exactlyOnce)
    (d : (Values L mem).OrderSelDec (mem ℓ) (K × V)) :
    ((Eager L mem).kassume_ordering u d).den = (Values L mem).kassume_ordering u.den d := rfl

theorem eag_kfold_early_stop_den {K V A : Type} [DecidableEq K] [DecidableEq V] [DecidableEq A]
    (ks : (Eager L mem).KeyedStream ℓ K V .totalOrder .exactlyOnce) (f : A → V → A × Bool) (init : A) :
    ((Eager L mem).kfold_early_stop ks f init).den
      = (Values L mem).kfold_early_stop ks.den f init := rfl

theorem eag_kget_max_key_den {α K V : Type} [DecidableEq α] [DecidableEq K] [DecidableEq V]
    [LinearOrder K] {ord : StrOrd} {ret : Retries}
    (ks : (Eager L mem).KeyedSingleton ℓ α K V ord ret .boundedValue) :
    ((Eager L mem).kget_max_key ks).den
      = (Values L mem).kget_max_key (ord := ord) (ret := ret) ks.den := rfl

theorem eag_tinto_keyed_den {K V : Type} [DecidableEq K] [DecidableEq V] {ord : StrOrd}
    (x : (Eager L mem).AtomicStream ℓ (K × V) ord .exactlyOnce) :
    ((Eager L mem).tinto_keyed x).den = (Values L mem).tinto_keyed x.den := rfl

theorem eag_reduce_watermark_den {K V : Type} [DecidableEq K] [DecidableEq V] [LinearOrder K]
    {ord : StrOrd}
    (ks : (Eager L mem).AtomicKeyedStream ℓ K V ord .exactlyOnce)
    (wm : (Eager L mem).Atomic ℓ (Option K)) (f : V → V → V)
    (ok : FoldOk ord .exactlyOnce (reduceStep f)) :
    ((Eager L mem).reduce_watermark ks wm f ok).den
      = (Values L mem).reduce_watermark ks.den wm.den f ok := rfl

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
    (t : (Eager L mem).Atomic ℓ (Option α))
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
    (s : (Eager L mem).Atomic ℓ α)
    (f : Fin (mem ℓ) → α → β) :
    ((Eager L mem).mapTick s f).den
      = (Values L mem).mapTick s.den f := rfl

theorem eag_zipTick_den {α β : Type}
    (a : (Eager L mem).Atomic ℓ α)
    (b : (Eager L mem).Atomic ℓ β) :
    ((Eager L mem).zipTick a b).den
      = (Values L mem).zipTick a.den b.den := rfl

/-! The shaped tick former at `Eager`: `.den` of ANY output-path
projection is the `Values` former's same projection on the `.den`
legs — definitionally (`pack` is shape-structural), for every path and
leaf kind. One simproc computes the path and emits the right-hand side
(D70; formerly a table of one rule per path and leaf kind, hardcoded to
right-nested outputs of four legs and blind to `.ksing` leaves). -/
section EagTickScanProj
open Lean Meta Simp

/-- A `Prod.fst`/`Prod.snd` chain down to its root; the path is listed
innermost first (`true` = `.1`). -/
partial def stripProdProjE (e : Expr) : Expr × List Bool :=
  if e.isAppOfArity ``Prod.fst 3 then
    let (r, p) := stripProdProjE (e.getArg! 2); (r, p ++ [true])
  else if e.isAppOfArity ``Prod.snd 3 then
    let (r, p) := stripProdProjE (e.getArg! 2); (r, p ++ [false])
  else (e.consumeMData, [])

/-- `eagTickScanProj`: `(π (tick_scan sts ins outs x g init)).den` at
`Eager` is `π ((Values L mem).tick_scan sts ins outs (EagerTick.den ins x) g init)`. -/
simproc_decl eagTickScanProj (_) := fun e => do
  let e := e.consumeMData
  let subj? : Option Expr :=
    match e with
    | .proj ``EPack 1 s => some s
    | _ => if e.isAppOfArity ``EPack.den 3 then some e.appArg! else none
  let some subj := subj? | return .continue
  let (root, path) := stripProdProjE subj
  unless root.isAppOfArity ``HydroSem.tick_scan 10 do return .continue
  let args := root.getAppArgs
  let H := args[2]!.consumeMData
  unless H.isAppOfArity ``Eager 2 do return .continue
  let hargs := H.getAppArgs
  let (ℓ, sts, ins, outs, x, g, init) :=
    (args[3]!, args[4]!, args[5]!, args[6]!, args[7]!, args[8]!, args[9]!)
  let vals := mkAppN (mkConst ``Values) #[hargs[0]!, hargs[1]!]
  -- `EagerTick.den {L mem ℓ} sh x`
  let legs := mkAppN (mkConst ``EagerTick.den) #[hargs[0]!, hargs[1]!, ℓ, ins, x]
  -- the former at `Values`: the same head constant over the `.den` legs
  let former := mkAppN root.getAppFn #[hargs[0]!, hargs[1]!, vals, ℓ, sts, ins, outs, legs, g, init]
  let rhs ← path.foldlM (fun acc isFst =>
    mkAppM (if isFst then ``Prod.fst else ``Prod.snd) #[acc]) former
  return .visit { expr := rhs }

end EagTickScanProj

/-! The `EagerTick` structural rules, at the interpretations' own carrier
types (what the simp unifier sees in a program's goals). -/
section EagerTickRules
variable {a b : TickShape} {τ : Type} {α : Type} {inst : DecidableEq α}
  {ord : StrOrd} {ret : Retries}
theorem eag_den_pair (x : AtomicOf (Eager L mem).Atomic (Eager L mem).AtomicStream (Eager L mem).AtomicKeyedSingleton ℓ (.pair a b)) :
    (EagerTick.den (.pair a b) x : AtomicOf (Values L mem).Atomic (Values L mem).AtomicStream (Values L mem).AtomicKeyedSingleton ℓ (.pair a b))
      = (EagerTick.den a x.1, EagerTick.den b x.2) := rfl
theorem eag_den_sing (x : (Eager L mem).Atomic ℓ τ) :
    (EagerTick.den (.sing τ) x : (Values L mem).Atomic ℓ τ) = x.den := rfl
theorem eag_den_stream (x : (Eager L mem).AtomicStream ℓ α ord ret) :
    (EagerTick.den (.stream α inst ord ret) x : (Values L mem).AtomicStream ℓ α ord ret) = x.den := rfl
theorem eag_den_ksing (K V : Type) (iK : DecidableEq K) (iV : DecidableEq V)
    (x : (Eager L mem).AtomicKeyedSingleton ℓ K V) :
    (EagerTick.den (.ksing K V iK iV) x : (Values L mem).AtomicKeyedSingleton ℓ K V) = x.den := rfl
end EagerTickRules

theorem eag_defer_tick_den {σ : Type} (init : σ)
    (t : (Eager L mem).Atomic ℓ σ) :
    ((Eager L mem).defer_tick init t).den
      = (Values L mem).defer_tick init t.den := rfl

theorem eag_allTicks_den {β : Type} [DecidableEq β] {ord : StrOrd}
    (bs : (Eager L mem).AtomicStream ℓ β ord .exactlyOnce) :
    ((Eager L mem).allTicks bs).den
      = (Values L mem).allTicks bs.den := rfl

theorem eag_flattenOrdered_den {β : Type} [DecidableEq β]
    (t : (Eager L mem).Atomic ℓ (List β)) :
    ((Eager L mem).flattenOrdered t).den
      = (Values L mem).flattenOrdered t.den := rfl

theorem eag_flattenUnordered_den {β : Type} [DecidableEq β]
    (t : (Eager L mem).Atomic ℓ (List β)) :
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
    (body : (Eager L mem).Atomic ℓ σ →
      (Eager L mem).Atomic ℓ σ) :
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
      eag_values_den, eag_into_keyed_den, eag_kassume_ordering_den,
      eag_kfold_early_stop_den, eag_kget_max_key_den, eag_tinto_keyed_den,
      eag_reduce_watermark_den, eag_weaken_retries_den, eag_union_den,
      eag_assume_ordering_den, eag_fold_den, eag_fold_monotone_den,
      eag_snapshot_unbounded_den, eag_snapshot_monotonic_den,
      eag_batch_den, eag_batch_ordered_den,
      eag_sample_every_den, eag_timeout_snapshot_den,
      eag_source_interval_batch_den,
      eag_mapTick_den, eag_zipTick_den,
      eag_defer_tick_den,
      eagTickScanProj,
      eag_den_pair, eag_den_sing, eag_den_stream, eag_den_ksing,
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
