import Hydro.Sched

/-!
# Machine-op causality (`SchedCausal`)

**Causality**: a machine op's output views at steps `≤ h` are a function of its inputs'
views at steps `≤ h`. This file proves it for every `SchedSem` op, one congruence lemma
(`causal_*`) per operator over the view-agreement relations `VAgree`/`SAgree`/`FAgree`/
`TAgree`. Totality over the signature matters: an op without a congruence lemma could
smuggle acausal information.

On top of the base lemmas, **graded agreement** (`SAgreeL`/`FAgreeL`/`TAgreeL`) tracks a
lag `n`: the wires agree at the steps `t` with `t + n ≤ h`. Immediate ops keep their inputs'
lag (the `max` of several); a network hop and `defer_tick` take one off because they read
one step back. The knot results follow: for a body strict in its knot wire (`SStrict`/
`TStrict`) the Kleene stages stabilize below their count (`stab_iterate_strict`), the
diagonal is a fixpoint of the body (`fix_stream_eq`/`fix_tick_eq`), and knots whose bodies are
graded-related are graded-related (`causalL_fix_stream`/`causalL_fix_tick`). Without
strictness, causally related bodies still give agreeing knots (`causal_fix_stream`/
`causal_fix_tick`).

The lemmas are heterogeneous: two different op-built composites of the same shape with
different leaf wires, so the same statements serve "one run at two stages" and "stage body
vs. diagonal body". Decisions are identical on both sides; causality is about wires. The
coupling corner's knot `wf` triple (`Hydro/Couple.lean`, which `hydro def` generates per
knot) rests on these results.
-/

namespace Hydro

/-! ## View-agreement relations, one per machine carrier shape -/

/-- Two step wires agree at all views up to `h`. -/
def VAgree {α : Type} (h : Nat) (x y : StepHist α) : Prop :=
  ∀ t, t ≤ h → x.view t = y.view t

/-- Stream carriers: member-wise view agreement up to `h`. -/
def SAgree {n : Nat} {α : Type} (h : Nat)
    (x y : Fin n → StepHist α) : Prop :=
  ∀ i, VAgree h (x i) (y i)

/-- Fold carriers (the live value per step): value agreement at all steps up to `h`. -/
def FAgree {n : Nat} {σ : Type} (h : Nat)
    (x y : Fin n → Nat → σ) : Prop :=
  ∀ i, ∀ t, t ≤ h → x i t = y i t

/-- Atomic carriers (`Atomic`/`AtomicStream`): trace agreement at all steps up to `h`. -/
def TAgree {n : Nat} {σ : Type} (h : Nat)
    (x y : Fin n → Nat → Trace σ) : Prop :=
  ∀ i, ∀ t, t ≤ h → x i t = y i t

theorem VAgree.refl {α : Type} (h : Nat) (x : StepHist α) :
    VAgree h x x := fun _ _ => rfl

theorem SAgree.refl {n : Nat} {α : Type} (h : Nat)
    (x : Fin n → StepHist α) : SAgree h x x := fun _ _ _ => rfl

theorem FAgree.refl {n : Nat} {σ : Type} (h : Nat)
    (x : Fin n → Nat → σ) : FAgree h x x := fun _ _ _ => rfl

theorem TAgree.refl {n : Nat} {σ : Type} (h : Nat)
    (x : Fin n → Nat → Trace σ) : TAgree h x x := fun _ _ _ => rfl

/-! Agreement at a horizon holds at every lower horizon. -/

theorem VAgree.mono {α : Type} {h h' : Nat} {x y : StepHist α}
    (hh : h' ≤ h) (hv : VAgree h x y) : VAgree h' x y :=
  fun t ht => hv t (ht.trans hh)

theorem SAgree.mono {n : Nat} {α : Type} {h h' : Nat}
    {x y : Fin n → StepHist α} (hh : h' ≤ h) (hs : SAgree h x y) :
    SAgree h' x y := fun i => (hs i).mono hh

theorem FAgree.mono {n : Nat} {σ : Type} {h h' : Nat}
    {x y : Fin n → Nat → σ} (hh : h' ≤ h) (hf : FAgree h x y) :
    FAgree h' x y := fun i t ht => hf i t (ht.trans hh)

theorem TAgree.mono {n : Nat} {σ : Type} {h h' : Nat}
    {x y : Fin n → Nat → Trace σ} (hh : h' ≤ h) (ht : TAgree h x y) :
    TAgree h' x y := fun i t htl => ht i t (htl.trans hh)

/-! ## Congruences over the machine primitives -/

/-- Increments agree strictly below the agreement horizon: `inc t` reads the views at `t`
and `t + 1`. -/
theorem VAgree.inc {α : Type} {h : Nat} {x y : StepHist α}
    (hv : VAgree h x y) {t : Nat} (ht : t + 1 ≤ h) :
    x.inc t = y.inc t := by
  show (x.view (t + 1)).drop (x.view t).length
    = (y.view (t + 1)).drop (y.view t).length
  rw [hv t (Nat.le_of_succ_le ht), hv (t + 1) ht]

/-- `merge2View` congruence below the horizon. -/
theorem merge2View_congr {α : Type} {h : Nat} {a b a' b' : StepHist α}
    (ha : VAgree h a a') (hb : VAgree h b b') :
    ∀ t, t ≤ h → merge2View a b t = merge2View a' b' t
  | 0, ht => by
    show a.view 0 ++ b.view 0 = a'.view 0 ++ b'.view 0
    rw [ha 0 ht, hb 0 ht]
  | t + 1, ht => by
    show merge2View a b t ++ a.inc t ++ b.inc t
      = merge2View a' b' t ++ a'.inc t ++ b'.inc t
    rw [merge2View_congr ha hb t (Nat.le_of_succ_le ht),
      ha.inc ht, hb.inc ht]

/-- `mergeNView` congruence below the horizon. -/
theorem mergeNView_congr {m : Nat} {α : Type} {h : Nat}
    {k k' : Fin m → StepHist α} (hk : ∀ j, VAgree h (k j) (k' j)) :
    ∀ t, t ≤ h → mergeNView k t = mergeNView k' t
  | 0, ht => by
    show (List.finRange m).flatMap (fun j => (k j).view 0)
      = (List.finRange m).flatMap (fun j => (k' j).view 0)
    rw [funext (fun j => hk j 0 ht)]
  | t + 1, ht => by
    show mergeNView k t ++ (List.finRange m).flatMap (fun j => (k j).inc t)
      = mergeNView k' t
        ++ (List.finRange m).flatMap (fun j => (k' j).inc t)
    rw [mergeNView_congr hk t (Nat.le_of_succ_le ht),
      funext (fun j => (hk j).inc ht)]

/-- A tick step recorded by `tickSteps p t` is at most `t`. -/
theorem tickSteps_mem_le {p : Nat → Bool} {t st : Nat}
    (hst : st ∈ tickSteps p t) : st ≤ t := by
  have := (List.mem_filter.mp hst).1
  exact Nat.lt_succ_iff.mp (List.mem_range.mp this)

/-- `batchesFrom` congruence: the sources agree at every step it walks. -/
theorem batchesFrom_congr {α : Type} {src src' : Nat → List α} :
    ∀ (ss : List Nat), (∀ s ∈ ss, src s = src' s) →
      ∀ c, batchesFrom src ss c = batchesFrom src' ss c
  | [], _, _ => rfl
  | s :: rest, hs, c => by
    show (src s).drop c :: batchesFrom src rest (src s).length = _
    rw [hs s (List.mem_cons_self ..),
      batchesFrom_congr rest (fun u hu => hs u (List.mem_cons_of_mem _ hu))]
    rfl

/-- `famFreeze` congruence below the horizon. -/
theorem famFreeze_congr_le {n : Nat} {β : Type} [DecidableEq β]
    {f g : Nat → Fin n → List β} :
    ∀ t, (∀ t' , t' ≤ t → f t' = g t') → famFreeze f t = famFreeze g t
  | 0, hfg => hfg 0 (Nat.le_refl _)
  | t + 1, hfg => by
    show (if (List.finRange n).all
        (fun i => ((famFreeze f t) i).isPrefixOf (f (t + 1) i))
      then f (t + 1) else famFreeze f t) = _
    rw [famFreeze_congr_le t (fun t' ht' => hfg t' (Nat.le_succ_of_le ht')),
      hfg (t + 1) (Nat.le_refl _)]
    rfl

/-! ## Per-op causality: stream and keyed ops

Each `causal_*` lemma unfolds the op at one member and step and rewrites with the input
agreement. -/

section Ops

variable {L : Type} {mem : L → Nat}
  {pacing : Pacing L mem}

theorem causal_map {ℓ : L} {α β : Type} [DecidableEq α] [DecidableEq β]
    {ord : StrOrd} {h : Nat}
    {s s' : (SchedSem L mem pacing).Stream ℓ α ord .exactlyOnce}
    (f : Fin (mem ℓ) → α → β) (hs : SAgree h s s') :
    SAgree h ((SchedSem L mem pacing).map s f)
      ((SchedSem L mem pacing).map s' f) := by
  intro i t ht
  show ((s i).view t).map (f i) = ((s' i).view t).map (f i)
  rw [hs i t ht]

theorem causal_filterMap {ℓ : L} {α β : Type} [DecidableEq α]
    [DecidableEq β] {ord : StrOrd} {h : Nat}
    {s s' : (SchedSem L mem pacing).Stream ℓ α ord .exactlyOnce}
    (f : Fin (mem ℓ) → α → Option β) (hs : SAgree h s s') :
    SAgree h ((SchedSem L mem pacing).filterMap s f)
      ((SchedSem L mem pacing).filterMap s' f) := by
  intro i t ht
  show ((s i).view t).filterMap (f i) = ((s' i).view t).filterMap (f i)
  rw [hs i t ht]

/-- A delivered, sender-tagged wire agrees below the horizon when the sender's wire does:
`gateView` reads the source strictly below the step. -/
theorem VAgree.deliver_tag {α β : Type} {h : Nat} {s s' : StepHist α}
    (hs : VAgree h s s') (p : Nat → Bool) (c : Nat → Nat) (f : α → β) :
    VAgree h ((s.deliver p c).map f) ((s'.deliver p c).map f) := by
  intro t ht
  show (gateView p (fun t => (s.view t).take (cumMax c (t + 1))) t).map f
    = (gateView p (fun t => (s'.view t).take (cumMax c (t + 1))) t).map f
  rw [gateView_congr p _ _ h (fun t' ht' => by rw [hs t' (Nat.le_of_lt ht')]) t ht]

theorem causal_broadcast_closed {c p : L} {α : Type} [DecidableEq α]
    {ord : StrOrd} {ret : Retries} {h : Nat}
    (d : Fin (mem p) → Fin (mem c) → Nat → Nat)
    {s s' : (SchedSem L mem pacing).Stream c α ord ret}
    (hs : SAgree h s s') :
    SAgree h ((SchedSem L mem pacing).broadcast_closed d s)
      ((SchedSem L mem pacing).broadcast_closed d s') := by
  intro i t ht
  show mergeNView (fun j => ((s j).deliver (pacing.active p i) (d i j)).map (fun v => (j, v))) t
    = mergeNView (fun j => ((s' j).deliver (pacing.active p i) (d i j)).map (fun v => (j, v))) t
  exact mergeNView_congr (fun j => (hs j).deliver_tag _ _ _) t ht

theorem causal_demux {c p : L} {α : Type} [DecidableEq α]
    {ord : StrOrd} {h : Nat}
    (d : Fin (mem p) → Fin (mem c) → Nat → Nat)
    {s s' : (SchedSem L mem pacing).Stream c (Nat × α) ord .exactlyOnce}
    (addr : Fin (mem p) → Nat) (hs : SAgree h s s') :
    SAgree h ((SchedSem L mem pacing).demux d s addr)
      ((SchedSem L mem pacing).demux d s' addr) := by
  intro i t ht
  show mergeNView (fun j => (((s j).filterMap _).deliver (pacing.active p i) (d i j)).map (fun v => (j, v))) t
    = mergeNView (fun j => (((s' j).filterMap _).deliver (pacing.active p i) (d i j)).map (fun v => (j, v))) t
  refine mergeNView_congr (fun j => VAgree.deliver_tag ?_ _ _ _) t ht
  intro t' ht'
  show ((s j).view t').filterMap _ = ((s' j).view t').filterMap _
  rw [hs j t' ht']

theorem causal_values {ℓ : L} {K V : Type} [DecidableEq K] [DecidableEq V]
    {ord : StrOrd} {ret : Retries} {h : Nat}
    {k k' : (SchedSem L mem pacing).KeyedStream ℓ K V ord ret}
    (hk : SAgree h k k') :
    SAgree h ((SchedSem L mem pacing).values k)
      ((SchedSem L mem pacing).values k') := by
  intro i t ht
  show ((k i).view t).map Prod.snd = ((k' i).view t).map Prod.snd
  rw [hk i t ht]

theorem causal_into_keyed {ℓ : L} {K V : Type} [DecidableEq K] [DecidableEq V]
    {ord : StrOrd} {h : Nat}
    {s s' : (SchedSem L mem pacing).Stream ℓ (K × V) ord .exactlyOnce}
    (hs : SAgree h s s') :
    SAgree h ((SchedSem L mem pacing).into_keyed s)
      ((SchedSem L mem pacing).into_keyed s') := hs

theorem causal_kassume_ordering {ℓ : L} {K V : Type} [DecidableEq K] [DecidableEq V]
    {h : Nat}
    {u u' : (SchedSem L mem pacing).KeyedStream ℓ K V .noOrder .exactlyOnce}
    (d : Unit) (hu : SAgree h u u') :
    SAgree h ((SchedSem L mem pacing).kassume_ordering u d)
      ((SchedSem L mem pacing).kassume_ordering u' d) := hu

theorem causal_kfold_early_stop {ℓ : L} {K V A : Type} [DecidableEq K]
    [DecidableEq V] [DecidableEq A] {h : Nat}
    {ks ks' : (SchedSem L mem pacing).KeyedStream ℓ K V .totalOrder .exactlyOnce}
    (f : A → V → A × Bool) (init : A) (hk : SAgree h ks ks') :
    FAgree h ((SchedSem L mem pacing).kfold_early_stop ks f init)
      ((SchedSem L mem pacing).kfold_early_stop ks' f init) := by
  intro i t ht
  show keyedFoldEarlyStopAssoc f init ((ks i).view t)
    = keyedFoldEarlyStopAssoc f init ((ks' i).view t)
  rw [hk i t ht]

theorem causal_kget_max_key {ℓ : L} {α K V : Type} [DecidableEq α] [DecidableEq K]
    [DecidableEq V] [LinearOrder K] {ord : StrOrd} {ret : Retries} {h : Nat}
    {ks ks' : (SchedSem L mem pacing).KeyedSingleton ℓ α K V ord ret .boundedValue}
    (hk : FAgree h ks ks') :
    FAgree h ((SchedSem L mem pacing).kget_max_key ks)
      ((SchedSem L mem pacing).kget_max_key ks') := by
  intro i t ht
  show (ks i t).maxEntry = (ks' i t).maxEntry
  rw [hk i t ht]

theorem causal_weaken_retries {ℓ : L} {α : Type} [DecidableEq α]
    {ord : StrOrd} {h : Nat}
    {s s' : (SchedSem L mem pacing).Stream ℓ α ord .exactlyOnce}
    (hs : SAgree h s s') :
    SAgree h ((SchedSem L mem pacing).weaken_retries s)
      ((SchedSem L mem pacing).weaken_retries s') := hs

theorem causal_union {ℓ : L} {α : Type} [DecidableEq α] {ret : Retries}
    {h : Nat}
    {a a' b b' : (SchedSem L mem pacing).Stream ℓ α .noOrder ret}
    (ha : SAgree h a a') (hb : SAgree h b b') :
    SAgree h ((SchedSem L mem pacing).union a b)
      ((SchedSem L mem pacing).union a' b') := by
  intro i t ht
  show merge2View (a i) (b i) t = merge2View (a' i) (b' i) t
  exact merge2View_congr (ha i) (hb i) t ht

theorem causal_assume_ordering {ℓ : L} {α : Type} [DecidableEq α]
    {h : Nat}
    {u u' : (SchedSem L mem pacing).Stream ℓ α .noOrder .exactlyOnce}
    (d : Unit) (hu : SAgree h u u') :
    SAgree h ((SchedSem L mem pacing).assume_ordering u d)
      ((SchedSem L mem pacing).assume_ordering u' d) := hu

theorem causal_fold {ℓ : L} {α σ : Type} [DecidableEq α]
    {ord : StrOrd} {ret : Retries} {h : Nat} (g : σ → α → σ) (init : σ)
    (Pre : α → α → Prop) [DecidableRel Pre] (ok : FoldOk ord ret Pre g)
    {s s' : (SchedSem L mem pacing).Stream ℓ α ord ret}
    (hs : SAgree h s s') :
    FAgree h ((SchedSem L mem pacing).fold g init Pre ok s)
      ((SchedSem L mem pacing).fold g init Pre ok s') := by
  intro i t ht
  show ((s i).view t).foldl g init = ((s' i).view t).foldl g init
  rw [hs i t ht]

theorem causal_fold_monotone {ℓ : L} {α σ : Type} [DecidableEq α]
    {ord : StrOrd} {ret : Retries} {h : Nat} (vo : ValueOrder σ)
    (g : σ → α → σ) (init : σ) (ok : FoldOk ord ret (fun _ _ => True) g)
    (hinfl : ∀ s x, vo.le s (g s x))
    {s s' : (SchedSem L mem pacing).Stream ℓ α ord ret}
    (hs : SAgree h s s') :
    FAgree h
      ((SchedSem L mem pacing).fold_monotone vo g init ok hinfl s)
      ((SchedSem L mem pacing).fold_monotone vo g init ok hinfl s') := by
  intro i t ht
  show ((s i).view t).foldl g init = ((s' i).view t).foldl g init
  rw [hs i t ht]

end Ops

/-! ## Per-op causality: tick-boundary and in-tick ops -/

section TickOps

variable {L : Type} {mem : L → Nat}
  {pacing : Pacing L mem}

theorem causal_snapshot {ℓ : L} {α σ : Type} [DecidableEq α]
    {ord : StrOrd} {ret : Retries} {b : SingBound σ} {h : Nat}
    {s s' : (SchedSem L mem pacing).Singleton ℓ α σ ord ret b}
    (d : Unit) (hs : FAgree h s s') :
    TAgree h ((SchedSem L mem pacing).snapshot s d)
      ((SchedSem L mem pacing).snapshot s' d) := by
  intro i t ht
  show (tickSteps (pacing.ticks ℓ i) t).map (fun st => s i st)
    = (tickSteps (pacing.ticks ℓ i) t).map (fun st => s' i st)
  refine List.map_congr_left (fun st hst => ?_)
  exact hs i st ((tickSteps_mem_le hst).trans ht)

theorem causal_batch {ℓ : L} {α : Type} [DecidableEq α] {h : Nat} {ord : StrOrd}
    {ret : Retries}
    {s s' : (SchedSem L mem pacing).Stream ℓ α ord ret}
    (d : Unit) (hs : SAgree h s s') :
    TAgree h ((SchedSem L mem pacing).batch s d)
      ((SchedSem L mem pacing).batch s' d) := by
  intro i t ht
  show (batchesFrom ((s i).view) (tickSteps (pacing.ticks ℓ i) t) 0).map List.toArray
    = (batchesFrom ((s' i).view) (tickSteps (pacing.ticks ℓ i) t) 0).map List.toArray
  rw [batchesFrom_congr _
    (fun st hst => hs i st ((tickSteps_mem_le hst).trans ht)) 0]

theorem causal_mapTick {ℓ : L} {α β : Type} {h : Nat}
    {t t' : (SchedSem L mem pacing).Atomic ℓ α}
    (f : Fin (mem ℓ) → α → β) (ht : TAgree h t t') :
    TAgree h ((SchedSem L mem pacing).mapTick t f)
      ((SchedSem L mem pacing).mapTick t' f) := by
  intro i u hu
  show (t i u).map (f i) = (t' i u).map (f i)
  rw [ht i u hu]

theorem causal_zipTick {ℓ : L} {α β : Type} {h : Nat}
    {a a' : (SchedSem L mem pacing).Atomic ℓ α}
    {b b' : (SchedSem L mem pacing).Atomic ℓ β}
    (ha : TAgree h a a') (hb : TAgree h b b') :
    TAgree h ((SchedSem L mem pacing).zipTick a b)
      ((SchedSem L mem pacing).zipTick a' b') := by
  intro i u hu
  show Trace.zip (a i u) (b i u) = Trace.zip (a' i u) (b' i u)
  rw [ha i u hu, hb i u hu]

theorem causal_defer_tick {ℓ : L} {σ : Type} {h : Nat} (init : σ)
    {t t' : (SchedSem L mem pacing).Atomic ℓ σ}
    (ht : TAgree h t t') :
    TAgree h ((SchedSem L mem pacing).defer_tick init t)
      ((SchedSem L mem pacing).defer_tick init t') := by
  intro i u hu
  show (init :: t i (u - 1)).take (deferReleased (pacing.ticks ℓ i) (fun k => (t i (k - 1)).length) u)
    = (init :: t' i (u - 1)).take (deferReleased (pacing.ticks ℓ i) (fun k => (t' i (k - 1)).length) u)
  rw [ht i (u - 1) (Nat.le_trans (Nat.sub_le u 1) hu),
    deferReleased_congr _ u (fun v hv => by rw [ht i (v - 1) (Nat.le_trans (Nat.sub_le v 1) (Nat.le_trans hv hu))])]

theorem causal_flattenOrdered {ℓ : L} {β : Type} [DecidableEq β] {h : Nat}
    {t t' : (SchedSem L mem pacing).Atomic ℓ (List β)}
    (ht : TAgree h t t') :
    TAgree h ((SchedSem L mem pacing).flattenOrdered t)
      ((SchedSem L mem pacing).flattenOrdered t') := by
  intro i k hk
  show (t i k).map List.toArray = (t' i k).map List.toArray
  rw [ht i k hk]

theorem causal_flattenUnordered {ℓ : L} {β : Type} [DecidableEq β]
    {h : Nat}
    {t t' : (SchedSem L mem pacing).Atomic ℓ (List β)}
    (ht : TAgree h t t') :
    TAgree h ((SchedSem L mem pacing).flattenUnordered t)
      ((SchedSem L mem pacing).flattenUnordered t') := by
  intro i k hk
  show (t i k).map List.toArray = (t' i k).map List.toArray
  rw [ht i k hk]

theorem causal_source_interval {ℓ : L} {h : Nat} (delay : Fin (mem ℓ) → Nat) (period : Nat)
    (d : TickScript (mem ℓ) Nat) :
    SAgree h ((SchedSem L mem pacing).source_interval (ℓ := ℓ) delay period d)
      ((SchedSem L mem pacing).source_interval (ℓ := ℓ) delay period d) :=
  fun _ _ _ => rfl

theorem causal_firstTick {ℓ : L} {α : Type} [DecidableEq α] {ret : Retries} {h : Nat}
    {t t' : (SchedSem L mem pacing).AtomicStream ℓ α .totalOrder ret}
    (ht : TAgree h t t') :
    TAgree h ((SchedSem L mem pacing).firstTick t)
      ((SchedSem L mem pacing).firstTick t') := by
  intro i k hk
  show (t i k).map (fun b => b[0]?) = (t' i k).map (fun b => b[0]?)
  rw [ht i k hk]

/-! ### The tick former

A tick's output at a step depends on that step's inputs only: leafwise agreement in gives
leafwise agreement out. -/

/-- Leafwise agreement up to `h` of two shaped wire tuples. -/
@[reducible] def TAgreeOf {ℓ : L} (h : Nat) : (sh : TickShape) →
    AtomicOf (SchedTick.Tk mem) (SchedTick.TS mem) (SchedTick.TKS mem) (SchedTick.TKStr mem) ℓ sh →
    AtomicOf (SchedTick.Tk mem) (SchedTick.TS mem) (SchedTick.TKS mem) (SchedTick.TKStr mem) ℓ sh → Prop
  | .sing _, x, y => TAgree h x y
  | .stream _ _ _ _, x, y => TAgree h x y
  | .ksing _ _ _ _, x, y => TAgree h x y
  | .kstream _ _ _ _ _ _, x, y => TAgree h x y
  | .pair a b, x, y => TAgreeOf h a x.1 y.1 ∧ TAgreeOf h b x.2 y.2

theorem TAgreeOf_fst {ℓ : L} {h : Nat} (a b : TickShape)
    {x y : AtomicOf (SchedTick.Tk mem) (SchedTick.TS mem) (SchedTick.TKS mem) (SchedTick.TKStr mem) ℓ (.pair a b)}
    (hxy : TAgreeOf h (.pair a b) x y) : TAgreeOf h a x.1 y.1 := hxy.1
theorem TAgreeOf_snd {ℓ : L} {h : Nat} (a b : TickShape)
    {x y : AtomicOf (SchedTick.Tk mem) (SchedTick.TS mem) (SchedTick.TKS mem) (SchedTick.TKStr mem) ℓ (.pair a b)}
    (hxy : TAgreeOf h (.pair a b) x y) : TAgreeOf h b x.2 y.2 := hxy.2

theorem SchedTick.slice_agree {ℓ : L} {h : Nat} : (sh : TickShape) →
    (x y : AtomicOf (SchedTick.Tk mem) (SchedTick.TS mem) (SchedTick.TKS mem) (SchedTick.TKStr mem) ℓ sh) →
    TAgreeOf h sh x y → ∀ i k, k ≤ h →
    SchedTick.slice sh x i k = SchedTick.slice sh y i k
  | .sing _, _, _, hxy, i, k, hk => hxy i k hk
  | .stream _ _ _ _, _, _, hxy, i, k, hk => hxy i k hk
  | .ksing _ _ _ _, _, _, hxy, i, k, hk => hxy i k hk
  | .kstream _ _ _ _ _ _, _, _, hxy, i, k, hk => hxy i k hk
  | .pair a b, x, y, hxy, i, k, hk => by
    show Trace.zip (SchedTick.slice a x.1 i k) (SchedTick.slice b x.2 i k)
      = Trace.zip (SchedTick.slice a y.1 i k) (SchedTick.slice b y.2 i k)
    rw [SchedTick.slice_agree a x.1 y.1 hxy.1 i k hk,
      SchedTick.slice_agree b x.2 y.2 hxy.2 i k hk]

theorem SchedTick.unslice_agree {ℓ : L} {h : Nat} : (sh : TickShape) →
    (F G : Fin (mem ℓ) → Nat → Trace (BoundedOf SchedTick.BS SchedTick.BStr SchedTick.BKS SchedTick.BKStr sh)) →
    (∀ i k, k ≤ h → F i k = G i k) →
    TAgreeOf h sh (SchedTick.unslice sh F) (SchedTick.unslice sh G)
  | .sing _, _, _, hfg => hfg
  | .stream _ _ _ _, _, _, hfg => hfg
  | .ksing _ _ _ _, _, _, hfg => hfg
  | .kstream _ _ _ _ _ _, _, _, hfg => hfg
  | .pair a b, F, G, hfg =>
    ⟨SchedTick.unslice_agree a _ _ (fun i k hk => by rw [hfg i k hk]),
     SchedTick.unslice_agree b _ _ (fun i k hk => by rw [hfg i k hk])⟩

theorem causal_tick_scan {ℓ : L} {h : Nat} (sts ins outs : TickShape)
    {x x' : AtomicOf (SchedTick.Tk mem) (SchedTick.TS mem) (SchedTick.TKS mem) (SchedTick.TKStr mem) ℓ ins}
    (g : Fin (mem ℓ) → BoundedOf SchedTick.BS SchedTick.BStr SchedTick.BKS SchedTick.BKStr sts →
      BoundedOf SchedTick.BS SchedTick.BStr SchedTick.BKS SchedTick.BKStr ins →
      BoundedOf SchedTick.BS SchedTick.BStr SchedTick.BKS SchedTick.BKStr sts × BoundedOf SchedTick.BS SchedTick.BStr SchedTick.BKS SchedTick.BKStr outs)
    (init : SeedOf sts) (hx : TAgreeOf h ins x x') :
    TAgreeOf h outs ((SchedSem L mem pacing).tick_scan sts ins outs x g init)
      ((SchedSem L mem pacing).tick_scan sts ins outs x' g init) :=
  SchedTick.unslice_agree outs _ _ (fun i k hk => by
    show scanAcrossTicksTrace (g i) (SchedTick.seed sts init) (SchedTick.slice ins x i k)
      = scanAcrossTicksTrace (g i) (SchedTick.seed sts init) (SchedTick.slice ins x' i k)
    rw [SchedTick.slice_agree ins x x' hx i k hk])

end TickOps

/-! ## Per-op causality: `famHist` producers and the remaining tick ops -/

section KnotOps

variable {L : Type} {mem : L → Nat}
  {pacing : Pacing L mem}

theorem causal_allTicks {ℓ : L} {β : Type} [DecidableEq β]
    {ord : StrOrd} {h : Nat}
    {bs bs' : (SchedSem L mem pacing).AtomicStream ℓ β ord .exactlyOnce}
    (hb : TAgree h bs bs') :
    SAgree h ((SchedSem L mem pacing).allTicks bs)
      ((SchedSem L mem pacing).allTicks bs') := by
  intro i u hu
  show famFreeze (fun step j => ((bs j step).map Array.toList).flatten) u i
    = famFreeze (fun step j => ((bs' j step).map Array.toList).flatten) u i
  rw [famFreeze_congr_le u (fun u' hu' => funext fun j => by
    rw [hb j u' (hu'.trans hu)])]

theorem causal_tinto_keyed {ℓ : L} {K V : Type} [DecidableEq K] [DecidableEq V]
    {ord : StrOrd} {h : Nat}
    {x x' : (SchedSem L mem pacing).AtomicStream ℓ (K × V) ord .exactlyOnce}
    (hx : TAgree h x x') :
    TAgree h ((SchedSem L mem pacing).tinto_keyed x)
      ((SchedSem L mem pacing).tinto_keyed x') := hx

theorem causal_tassume_ordering {ℓ : L} {α : Type} [DecidableEq α]
    {h : Nat}
    {u u' : (SchedSem L mem pacing).AtomicStream ℓ α .noOrder .exactlyOnce}
    (d : Unit) (hu : TAgree h u u') :
    TAgree h ((SchedSem L mem pacing).tassume_ordering u d)
      ((SchedSem L mem pacing).tassume_ordering u' d) := hu

theorem causal_reduce_watermark {ℓ : L} {K V : Type} [DecidableEq K] [Hashable K] [DecidableEq V]
    [LinearOrder K] {ord : StrOrd} {h : Nat}
    {ks ks' : (SchedSem L mem pacing).AtomicKeyedStream ℓ K V ord .exactlyOnce}
    {wm wm' : (SchedSem L mem pacing).Atomic ℓ (Option K)}
    (f : V → V → V) (Pre : V → V → Prop) [DecidableRel Pre]
    (ok : FoldOk ord .exactlyOnce Pre (reduceStep f))
    (hk : TAgree h ks ks') (hw : TAgree h wm wm') :
    TAgree h ((SchedSem L mem pacing).reduce_watermark ks wm f Pre ok)
      ((SchedSem L mem pacing).reduce_watermark ks' wm' f Pre ok) := by
  intro i t ht
  show scanAcrossTicksTrace (reduceWatermarkEager f) (SchedKMap.empty, none)
      (Trace.zip ((ks i t).map Array.toList) (wm i t))
    = scanAcrossTicksTrace (reduceWatermarkEager f) (SchedKMap.empty, none)
      (Trace.zip ((ks' i t).map Array.toList) (wm' i t))
  rw [hk i t ht, hw i t ht]

/-! The keyed-stream and keyed-singleton vocabulary. -/

theorem causal_kentries {ℓ : L} {K V : Type} [DecidableEq K] [DecidableEq V]
    {ord : StrOrd} {h : Nat}
    {ks ks' : (SchedSem L mem pacing).KeyedStream ℓ K V ord .exactlyOnce}
    (hk : SAgree h ks ks') :
    SAgree h ((SchedSem L mem pacing).kentries ks)
      ((SchedSem L mem pacing).kentries ks') := hk

theorem causal_kmap {ℓ : L} {K V W : Type} [DecidableEq K] [DecidableEq V] [DecidableEq W]
    {ord : StrOrd} {h : Nat}
    {ks ks' : (SchedSem L mem pacing).KeyedStream ℓ K V ord .exactlyOnce}
    (f : Fin (mem ℓ) → V → W) (hk : SAgree h ks ks') :
    SAgree h ((SchedSem L mem pacing).kmap ks f)
      ((SchedSem L mem pacing).kmap ks' f) := by
  intro i t ht
  show ((ks i).view t).map _ = ((ks' i).view t).map _
  rw [hk i t ht]

theorem causal_kallTicks {ℓ : L} {K V : Type} [DecidableEq K] [DecidableEq V]
    {ord : StrOrd} {h : Nat}
    {bs bs' : (SchedSem L mem pacing).AtomicKeyedStream ℓ K V ord .exactlyOnce}
    (hb : TAgree h bs bs') :
    SAgree h ((SchedSem L mem pacing).kallTicks bs)
      ((SchedSem L mem pacing).kallTicks bs') := by
  intro i u hu
  show famFreeze (fun step j => ((bs j step).map Array.toList).flatten) u i
    = famFreeze (fun step j => ((bs' j step).map Array.toList).flatten) u i
  rw [famFreeze_congr_le u (fun u' hu' => funext fun j => by
    rw [hb j u' (hu'.trans hu)])]

theorem causal_kfold {ℓ : L} {K V A : Type} [DecidableEq K] [Hashable K]
    [DecidableEq V] [DecidableEq A] {ord : StrOrd} {h : Nat} (g : A → V → A) (init : A)
    (Pre : V → V → Prop) [DecidableRel Pre] (ok : FoldOk ord .exactlyOnce Pre g)
    {ks ks' : (SchedSem L mem pacing).KeyedStream ℓ K V ord .exactlyOnce}
    (hk : SAgree h ks ks') :
    FAgree h ((SchedSem L mem pacing).kfold g init Pre ok ks)
      ((SchedSem L mem pacing).kfold g init Pre ok ks') := by
  intro i t ht
  show keyedFoldAssocH g init ((ks i).view t) = keyedFoldAssocH g init ((ks' i).view t)
  rw [hk i t ht]

theorem causal_ksnapshot {ℓ : L} {α K V : Type} [DecidableEq α] [DecidableEq K]
    [DecidableEq V] {ord : StrOrd} {ret : Retries} {b : KSBound} {h : Nat}
    {ks ks' : (SchedSem L mem pacing).KeyedSingleton ℓ α K V ord ret b}
    (d : Unit) (hk : FAgree h ks ks') :
    TAgree h ((SchedSem L mem pacing).ksnapshot ks d)
      ((SchedSem L mem pacing).ksnapshot ks' d) := by
  intro i t ht
  show (tickSteps (pacing.ticks ℓ i) t).map (fun st => ks i st)
    = (tickSteps (pacing.ticks ℓ i) t).map (fun st => ks' i st)
  refine List.map_congr_left (fun st hst => ?_)
  exact hk i st ((tickSteps_mem_le hst).trans ht)

theorem causal_tfold {ℓ : L} {α σ : Type} [DecidableEq α] {ord : StrOrd} {ret : Retries}
    {h : Nat} (g : σ → α → σ) (init : σ) (Pre : α → α → Prop) [DecidableRel Pre]
    (ok : FoldOk ord ret Pre g)
    {bs bs' : (SchedSem L mem pacing).AtomicStream ℓ α ord ret}
    (hb : TAgree h bs bs') :
    TAgree h ((SchedSem L mem pacing).tfold g init Pre ok bs)
      ((SchedSem L mem pacing).tfold g init Pre ok bs') := by
  intro i t ht
  show tfoldHist g init (bs i t) = tfoldHist g init (bs' i t)
  rw [hb i t ht]

theorem causal_latest {ℓ : L} {σ : Type} [DecidableEq σ] {h : Nat}
    {t t' : (SchedSem L mem pacing).Atomic ℓ (Option σ)}
    (ht : TAgree h t t') :
    FAgree h ((SchedSem L mem pacing).latest t)
      ((SchedSem L mem pacing).latest t') := by
  intro i u hu
  show lastUpdate (famFreeze (fun s j => t j s) u i) = lastUpdate (famFreeze (fun s j => t' j s) u i)
  rw [famFreeze_congr_le u (fun u' hu' => funext fun j => by
    rw [ht j u' (hu'.trans hu)])]

theorem causal_latest_monotone {ℓ : L} {h : Nat}
    {t t' : (SchedSem L mem pacing).Atomic ℓ (Option Nat)}
    (ht : TAgree h t t') :
    FAgree h ((SchedSem L mem pacing).latest_monotone t)
      ((SchedSem L mem pacing).latest_monotone t') := by
  intro i u hu
  show lastUpdate (famFreeze (fun s j => t j s) u i) = lastUpdate (famFreeze (fun s j => t' j s) u i)
  rw [famFreeze_congr_le u (fun u' hu' => funext fun j => by
    rw [ht j u' (hu'.trans hu)])]

theorem causal_time {ℓ : L} {h : Nat} (d : TickScript (mem ℓ) Nat) :
    TAgree h ((SchedSem L mem pacing).time (ℓ := ℓ) d)
      ((SchedSem L mem pacing).time (ℓ := ℓ) d) :=
  fun _ _ _ => rfl

theorem causal_snapshot_bounded {ℓ : L} {σ : Type} {h : Nat} (x : σ) (d : Unit) :
    TAgree h ((SchedSem L mem pacing).snapshot_bounded (ℓ := ℓ) x d)
      ((SchedSem L mem pacing).snapshot_bounded (ℓ := ℓ) x d) :=
  fun _ _ _ => rfl

end KnotOps

/-! ## Graded agreement

Agreement at a lag `n` below the horizon: the two wires agree at the steps `t` with
`t + n ≤ h`. Lag `0` is the plain agreement above; lag `1` is "strictly below the horizon".
Every immediate op keeps its inputs' lag (the `max` of several); a network hop and
`defer_tick` take one off because they read one step back; a knot's output lag is its body's
at a loop input of lag `L + 1`. The corner's lag arithmetic (`Hydro/Couple.lean`) follows
these rules; this file states them on the machine alone. -/

section Graded

variable {L : Type} {mem : L → Nat} {pacing : Pacing L mem}

/-- Stream carriers at lag `n`. -/
def SAgreeL {m : Nat} {α : Type} (h n : Nat) (x y : Fin m → StepHist α) : Prop :=
  ∀ i t, t + n ≤ h → (x i).view t = (y i).view t

/-- Per-step carriers (fold values, tick traces, keyed maps) at lag `n`. -/
def FAgreeL {m : Nat} {σ : Type} (h n : Nat) (x y : Fin m → Nat → σ) : Prop :=
  ∀ i t, t + n ≤ h → x i t = y i t

/-- Atomic carriers at lag `n`: `FAgreeL` at trace type. -/
abbrev TAgreeL {m : Nat} {σ : Type} (h n : Nat) (x y : Fin m → Nat → Trace σ) : Prop :=
  FAgreeL h n x y

theorem SAgreeL_zero {m : Nat} {α : Type} {h : Nat} {x y : Fin m → StepHist α} :
    SAgreeL h 0 x y ↔ SAgree h x y :=
  ⟨fun hs i t ht => hs i t (by omega), fun hs i t ht => hs i t (by omega)⟩

theorem FAgreeL_zero {m : Nat} {σ : Type} {h : Nat} {x y : Fin m → Nat → σ} :
    FAgreeL h 0 x y ↔ FAgree h x y :=
  ⟨fun hs i t ht => hs i t (by omega), fun hs i t ht => hs i t (by omega)⟩

theorem TAgreeL_zero {m : Nat} {σ : Type} {h : Nat} {x y : Fin m → Nat → Trace σ} :
    TAgreeL h 0 x y ↔ TAgree h x y := FAgreeL_zero

/-! The two directions as their own `theorem`s, so the knot generator (`HydroGenKnot`) can
apply them by name: `agreeLZeroOf cf ++ \`mp` and `++ \`mpr`. -/
theorem SAgreeL_zero.mp {m : Nat} {α : Type} {h : Nat} {x y : Fin m → StepHist α} (hs : SAgreeL h 0 x y) :
    SAgree h x y := SAgreeL_zero.1 hs
theorem SAgreeL_zero.mpr {m : Nat} {α : Type} {h : Nat} {x y : Fin m → StepHist α} (hs : SAgree h x y) :
    SAgreeL h 0 x y := SAgreeL_zero.2 hs
theorem FAgreeL_zero.mp {m : Nat} {σ : Type} {h : Nat} {x y : Fin m → Nat → σ} (hs : FAgreeL h 0 x y) :
    FAgree h x y := FAgreeL_zero.1 hs
theorem FAgreeL_zero.mpr {m : Nat} {σ : Type} {h : Nat} {x y : Fin m → Nat → σ} (hs : FAgree h x y) :
    FAgreeL h 0 x y := FAgreeL_zero.2 hs
theorem TAgreeL_zero.mp {m : Nat} {σ : Type} {h : Nat} {x y : Fin m → Nat → Trace σ} (hs : TAgreeL h 0 x y) :
    TAgree h x y := TAgreeL_zero.1 hs
theorem TAgreeL_zero.mpr {m : Nat} {σ : Type} {h : Nat} {x y : Fin m → Nat → Trace σ} (hs : TAgree h x y) :
    TAgreeL h 0 x y := TAgreeL_zero.2 hs

theorem SAgreeL.refl {m : Nat} {α : Type} (h n : Nat) (x : Fin m → StepHist α) :
    SAgreeL h n x x := fun _ _ _ => rfl
theorem FAgreeL.refl {m : Nat} {σ : Type} (h n : Nat) (x : Fin m → Nat → σ) :
    FAgreeL h n x x := fun _ _ _ => rfl

/-- A larger lag is a weaker claim. -/
theorem SAgreeL.mono_lag {m : Nat} {α : Type} {h n n' : Nat} {x y : Fin m → StepHist α}
    (hn : n ≤ n') (hs : SAgreeL h n x y) : SAgreeL h n' x y :=
  fun i t ht => hs i t (by omega)
theorem FAgreeL.mono_lag {m : Nat} {σ : Type} {h n n' : Nat} {x y : Fin m → Nat → σ}
    (hn : n ≤ n') (hs : FAgreeL h n x y) : FAgreeL h n' x y :=
  fun i t ht => hs i t (by omega)

/-- A lower horizon is a weaker claim. -/
theorem SAgreeL.mono {m : Nat} {α : Type} {h h' n : Nat} {x y : Fin m → StepHist α}
    (hh : h' ≤ h) (hs : SAgreeL h n x y) : SAgreeL h' n x y :=
  fun i t ht => hs i t (by omega)
theorem FAgreeL.mono {m : Nat} {σ : Type} {h h' n : Nat} {x y : Fin m → Nat → σ}
    (hh : h' ≤ h) (hs : FAgreeL h n x y) : FAgreeL h' n x y :=
  fun i t ht => hs i t (by omega)
theorem TAgreeL.mono_lag {m : Nat} {σ : Type} {h n n' : Nat} {x y : Fin m → Nat → Trace σ}
    (hn : n ≤ n') (hs : TAgreeL h n x y) : TAgreeL h n' x y :=
  FAgreeL.mono_lag hn hs
theorem TAgreeL.mono {m : Nat} {σ : Type} {h h' n : Nat} {x y : Fin m → Nat → Trace σ}
    (hh : h' ≤ h) (hs : TAgreeL h n x y) : TAgreeL h' n x y :=
  FAgreeL.mono hh hs
theorem TAgreeL.refl {m : Nat} {σ : Type} (h n : Nat) (x : Fin m → Nat → Trace σ) :
    TAgreeL h n x x := fun _ _ _ => rfl

/-- Lag `n` at horizon `h` is lag `0` at horizon `h - n`; the conversion needs `n ≤ h`. -/
theorem SAgreeL.to_base {m : Nat} {α : Type} {h n : Nat} {x y : Fin m → StepHist α}
    (hn : n ≤ h) (hs : SAgreeL h n x y) : SAgree (h - n) x y :=
  fun i t ht => hs i t (by omega)
theorem SAgreeL.of_base {m : Nat} {α : Type} {h n : Nat} {x y : Fin m → StepHist α}
    (hn : n ≤ h) (hs : SAgree (h - n) x y) : SAgreeL h n x y :=
  fun i t ht => hs i t (by omega)
theorem FAgreeL.to_base {m : Nat} {σ : Type} {h n : Nat} {x y : Fin m → Nat → σ}
    (hn : n ≤ h) (hs : FAgreeL h n x y) : FAgree (h - n) x y :=
  fun i t ht => hs i t (by omega)
theorem FAgreeL.of_base {m : Nat} {σ : Type} {h n : Nat} {x y : Fin m → Nat → σ}
    (hn : n ≤ h) (hs : FAgree (h - n) x y) : FAgreeL h n x y :=
  fun i t ht => hs i t (by omega)

/-! ### Lifting the per-op congruences to graded ones

An op causal at every horizon is causal at every lag: apply the base fact at the horizon
`h - n`. Two inputs meet at the `max` of their lags. A strict op (one that reads one step
back and emits nothing at step `0`) takes one off. The `lift*` lemmas are named by the
relation pair they convert (`S` for `SAgreeL`, `F` for `FAgreeL`). -/

theorem liftSS {m m' : Nat} {α β : Type} {F : (Fin m → StepHist α) → (Fin m' → StepHist β)}
    (hF : ∀ h x y, SAgree h x y → SAgree h (F x) (F y))
    {h n : Nat} {x y : Fin m → StepHist α} (hx : SAgreeL h n x y) : SAgreeL h n (F x) (F y) := by
  intro i t ht
  exact hF (h - n) x y (hx.to_base (by omega)) i t (by omega)

theorem liftSF {m m' : Nat} {α σ : Type} {F : (Fin m → StepHist α) → (Fin m' → Nat → σ)}
    (hF : ∀ h x y, SAgree h x y → FAgree h (F x) (F y))
    {h n : Nat} {x y : Fin m → StepHist α} (hx : SAgreeL h n x y) : FAgreeL h n (F x) (F y) := by
  intro i t ht
  exact hF (h - n) x y (hx.to_base (by omega)) i t (by omega)

theorem liftFF {m m' : Nat} {σ τ : Type} {F : (Fin m → Nat → σ) → (Fin m' → Nat → τ)}
    (hF : ∀ h x y, FAgree h x y → FAgree h (F x) (F y))
    {h n : Nat} {x y : Fin m → Nat → σ} (hx : FAgreeL h n x y) : FAgreeL h n (F x) (F y) := by
  intro i t ht
  exact hF (h - n) x y (hx.to_base (by omega)) i t (by omega)

theorem liftFS {m m' : Nat} {σ β : Type} {F : (Fin m → Nat → σ) → (Fin m' → StepHist β)}
    (hF : ∀ h x y, FAgree h x y → SAgree h (F x) (F y))
    {h n : Nat} {x y : Fin m → Nat → σ} (hx : FAgreeL h n x y) : SAgreeL h n (F x) (F y) := by
  intro i t ht
  exact hF (h - n) x y (hx.to_base (by omega)) i t (by omega)

theorem liftSS2 {m₁ m₂ m' : Nat} {α β γ : Type}
    {F : (Fin m₁ → StepHist α) → (Fin m₂ → StepHist β) → (Fin m' → StepHist γ)}
    (hF : ∀ h a a' b b', SAgree h a a' → SAgree h b b' → SAgree h (F a b) (F a' b'))
    {h n n' : Nat} {a a' : Fin m₁ → StepHist α} {b b' : Fin m₂ → StepHist β}
    (ha : SAgreeL h n a a') (hb : SAgreeL h n' b b') : SAgreeL h (max n n') (F a b) (F a' b') := by
  intro i t ht
  exact hF (h - max n n') a a' b b' ((ha.mono_lag (Nat.le_max_left _ _)).to_base (by omega))
    ((hb.mono_lag (Nat.le_max_right _ _)).to_base (by omega)) i t (by omega)

theorem liftFF2 {m₁ m₂ m' : Nat} {σ τ υ : Type}
    {F : (Fin m₁ → Nat → σ) → (Fin m₂ → Nat → τ) → (Fin m' → Nat → υ)}
    (hF : ∀ h a a' b b', FAgree h a a' → FAgree h b b' → FAgree h (F a b) (F a' b'))
    {h n n' : Nat} {a a' : Fin m₁ → Nat → σ} {b b' : Fin m₂ → Nat → τ}
    (ha : FAgreeL h n a a') (hb : FAgreeL h n' b b') : FAgreeL h (max n n') (F a b) (F a' b') := by
  intro i t ht
  exact hF (h - max n n') a a' b b' ((ha.mono_lag (Nat.le_max_left _ _)).to_base (by omega))
    ((hb.mono_lag (Nat.le_max_right _ _)).to_base (by omega)) i t (by omega)

/-- A strict stream op takes one off the lag. The hypothesis `hF` says agreement at `≤ h`
gives agreement at `≤ h + 1`, and `h0` that step `0` agrees outright. -/
theorem liftStrictSS {m m' : Nat} {α β : Type} {F : (Fin m → StepHist α) → (Fin m' → StepHist β)}
    (hF : ∀ h x y, SAgree h x y → SAgree (h + 1) (F x) (F y))
    (h0 : ∀ x y i, (F x i).view 0 = (F y i).view 0)
    {h n : Nat} {x y : Fin m → StepHist α} (hx : SAgreeL h n x y) : SAgreeL h (n - 1) (F x) (F y) := by
  intro i t ht
  cases n with
  | zero => exact hF h x y (SAgreeL_zero.mp hx) i t (by omega)
  | succ n =>
    cases t with
    | zero => exact h0 x y i
    | succ t => exact hF (h - (n + 1)) x y (hx.to_base (by omega)) i (t + 1) (by omega)

/-- `liftStrictSS` for per-step carriers. -/
theorem liftStrictFF {m m' : Nat} {σ τ : Type} {F : (Fin m → Nat → σ) → (Fin m' → Nat → τ)}
    (hF : ∀ h x y, FAgree h x y → FAgree (h + 1) (F x) (F y))
    (h0 : ∀ x y i, F x i 0 = F y i 0)
    {h n : Nat} {x y : Fin m → Nat → σ} (hx : FAgreeL h n x y) : FAgreeL h (n - 1) (F x) (F y) := by
  intro i t ht
  cases n with
  | zero => exact hF h x y (FAgreeL_zero.mp hx) i t (by omega)
  | succ n =>
    cases t with
    | zero => exact h0 x y i
    | succ t => exact hF (h - (n + 1)) x y (hx.to_base (by omega)) i (t + 1) (by omega)

/-! ### The strict ops' base facts -/

/-- Delivery reads the source one step back, so agreement at `≤ h` gives agreement at
`≤ h + 1`. -/
theorem VAgree.deliver_tag_strict {α β : Type} {h : Nat} {s s' : StepHist α}
    (hs : VAgree h s s') (p : Nat → Bool) (c : Nat → Nat) (f : α → β) :
    VAgree (h + 1) ((s.deliver p c).map f) ((s'.deliver p c).map f) := by
  intro t ht
  show (gateView p (fun t => (s.view t).take (cumMax c (t + 1))) t).map f
    = (gateView p (fun t => (s'.view t).take (cumMax c (t + 1))) t).map f
  rw [gateView_congr p _ _ (h + 1) (fun t' ht' => by rw [hs t' (Nat.le_of_lt_succ ht')]) t ht]

theorem strict_broadcast_closed {c p : L} {α : Type} [DecidableEq α]
    {ord : StrOrd} {ret : Retries} {h : Nat}
    (d : Fin (mem p) → Fin (mem c) → Nat → Nat)
    {s s' : (SchedSem L mem pacing).Stream c α ord ret}
    (hs : SAgree h s s') :
    SAgree (h + 1) ((SchedSem L mem pacing).broadcast_closed d s)
      ((SchedSem L mem pacing).broadcast_closed d s') := by
  intro i t ht
  show mergeNView (fun j => ((s j).deliver (pacing.active p i) (d i j)).map (fun v => (j, v))) t
    = mergeNView (fun j => ((s' j).deliver (pacing.active p i) (d i j)).map (fun v => (j, v))) t
  exact mergeNView_congr (fun j => (hs j).deliver_tag_strict _ _ _) t ht

theorem strict_demux {c p : L} {α : Type} [DecidableEq α]
    {ord : StrOrd} {h : Nat}
    (d : Fin (mem p) → Fin (mem c) → Nat → Nat)
    {s s' : (SchedSem L mem pacing).Stream c (Nat × α) ord .exactlyOnce}
    (addr : Fin (mem p) → Nat) (hs : SAgree h s s') :
    SAgree (h + 1) ((SchedSem L mem pacing).demux d s addr)
      ((SchedSem L mem pacing).demux d s' addr) := by
  intro i t ht
  show mergeNView (fun j => (((s j).filterMap _).deliver (pacing.active p i) (d i j)).map (fun v => (j, v))) t
    = mergeNView (fun j => (((s' j).filterMap _).deliver (pacing.active p i) (d i j)).map (fun v => (j, v))) t
  refine mergeNView_congr (fun j => VAgree.deliver_tag_strict ?_ _ _ _) t ht
  intro t' ht'
  show ((s j).view t').filterMap _ = ((s' j).view t').filterMap _
  rw [hs j t' ht']

/-- Nothing is delivered at step `0`. -/
theorem mergeNView_deliver_zero {m : Nat} {α β : Type} (k : Fin m → StepHist α)
    (p : Nat → Bool) (c : Fin m → Nat → Nat) (f : Fin m → α → β) :
    mergeNView (fun j => ((k j).deliver p (c j)).map (f j)) 0 = [] := by
  show (List.finRange m).flatMap (fun j => (((k j).deliver p (c j)).map (f j)).view 0) = []
  simp only [List.flatMap_eq_nil_iff]
  intro j _
  rfl

theorem broadcast_closed_zero {c p : L} {α : Type} [DecidableEq α]
    {ord : StrOrd} {ret : Retries}
    (d : Fin (mem p) → Fin (mem c) → Nat → Nat)
    (s : (SchedSem L mem pacing).Stream c α ord ret) (i : Fin (mem p)) :
    (((SchedSem L mem pacing).broadcast_closed d s) i).view 0 = [] :=
  mergeNView_deliver_zero (fun j => s j) (pacing.active p i) (d i) (fun j v => (j, v))

theorem demux_zero {c p : L} {α : Type} [DecidableEq α] {ord : StrOrd}
    (d : Fin (mem p) → Fin (mem c) → Nat → Nat)
    (s : (SchedSem L mem pacing).Stream c (Nat × α) ord .exactlyOnce)
    (addr : Fin (mem p) → Nat) (i : Fin (mem p)) :
    (((SchedSem L mem pacing).demux d s addr) i).view 0 = [] :=
  mergeNView_deliver_zero (fun j => (s j).filterMap _) (pacing.active p i) (d i) (fun j v => (j, v))

/-- `defer_tick` reads its input one step back (release before push), so agreement at `≤ h`
gives agreement at `≤ h + 1`. -/
theorem strict_defer_tick {ℓ : L} {σ : Type} {h : Nat} (init : σ)
    {t t' : (SchedSem L mem pacing).Atomic ℓ σ}
    (ht : TAgree h t t') :
    TAgree (h + 1) ((SchedSem L mem pacing).defer_tick init t)
      ((SchedSem L mem pacing).defer_tick init t') := by
  intro i u hu
  show (init :: t i (u - 1)).take (deferReleased (pacing.ticks ℓ i) (fun k => (t i (k - 1)).length) u)
    = (init :: t' i (u - 1)).take (deferReleased (pacing.ticks ℓ i) (fun k => (t' i (k - 1)).length) u)
  rw [ht i (u - 1) (by omega),
    deferReleased_congr _ u (fun v hv => by rw [ht i (v - 1) (by omega)])]

/-- `defer_tick` at step `0` does not depend on its input: the buffer is `init :: _` and at
most its head is released. -/
theorem defer_tick_zero {ℓ : L} {σ : Type} (init : σ)
    (t t' : (SchedSem L mem pacing).Atomic ℓ σ) (i : Fin (mem ℓ)) :
    (SchedSem L mem pacing).defer_tick init t i 0 = (SchedSem L mem pacing).defer_tick init t' i 0 := by
  show (init :: t i 0).take (if pacing.ticks ℓ i 0 then 1 else 0)
    = (init :: t' i 0).take (if pacing.ticks ℓ i 0 then 1 else 0)
  split <;> rfl

/-! ### The graded congruences for the strict ops and the simplest immediate ones -/

theorem causalL_map {ℓ : L} {α β : Type} [DecidableEq α] [DecidableEq β] {ord : StrOrd} {h n : Nat}
    {s s' : (SchedSem L mem pacing).Stream ℓ α ord .exactlyOnce} (f : Fin (mem ℓ) → α → β)
    (hs : SAgreeL h n s s') : SAgreeL h n ((SchedSem L mem pacing).map s f) ((SchedSem L mem pacing).map s' f) :=
  liftSS (fun _ _ _ hx => causal_map f hx) hs

theorem causalL_filterMap {ℓ : L} {α β : Type} [DecidableEq α] [DecidableEq β] {ord : StrOrd} {h n : Nat}
    {s s' : (SchedSem L mem pacing).Stream ℓ α ord .exactlyOnce} (f : Fin (mem ℓ) → α → Option β)
    (hs : SAgreeL h n s s') :
    SAgreeL h n ((SchedSem L mem pacing).filterMap s f) ((SchedSem L mem pacing).filterMap s' f) :=
  liftSS (fun _ _ _ hx => causal_filterMap f hx) hs

theorem causalL_broadcast_closed {c p : L} {α : Type} [DecidableEq α] {ord : StrOrd} {ret : Retries} {h n : Nat}
    (d : Fin (mem p) → Fin (mem c) → Nat → Nat)
    {s s' : (SchedSem L mem pacing).Stream c α ord ret} (hs : SAgreeL h n s s') :
    SAgreeL h (n - 1) ((SchedSem L mem pacing).broadcast_closed d s)
      ((SchedSem L mem pacing).broadcast_closed d s') :=
  liftStrictSS (fun _ _ _ hx => strict_broadcast_closed d hx)
    (fun x y i => by rw [broadcast_closed_zero d x i, broadcast_closed_zero d y i]) hs

theorem causalL_demux {c p : L} {α : Type} [DecidableEq α] {ord : StrOrd} {h n : Nat}
    (d : Fin (mem p) → Fin (mem c) → Nat → Nat)
    {s s' : (SchedSem L mem pacing).Stream c (Nat × α) ord .exactlyOnce}
    (addr : Fin (mem p) → Nat) (hs : SAgreeL h n s s') :
    SAgreeL h (n - 1) ((SchedSem L mem pacing).demux d s addr)
      ((SchedSem L mem pacing).demux d s' addr) :=
  liftStrictSS (fun _ _ _ hx => strict_demux d addr hx)
    (fun x y i => by rw [demux_zero d x addr i, demux_zero d y addr i]) hs

theorem causalL_defer_tick {ℓ : L} {σ : Type} {h n : Nat} (init : σ)
    {t t' : (SchedSem L mem pacing).Atomic ℓ σ} (ht : TAgreeL h n t t') :
    TAgreeL h (n - 1) ((SchedSem L mem pacing).defer_tick init t)
      ((SchedSem L mem pacing).defer_tick init t') :=
  liftStrictFF (fun _ _ _ hx => strict_defer_tick init hx)
    (fun x y i => defer_tick_zero init x y i) ht

end Graded

/-! ## Knots: the ungated diagonal of a strict body

`fix_stream`/`fix_tick` are defined as the Kleene diagonal `t ↦ (body^[t+1] ⊥) t` with no
gate. A body
is **strict** in the knot wire when agreement at lag `L + 1` in gives agreement at lag `L`
out: the output at a step is determined by the wire strictly before it, because every path
from the wire to the output passes a network hop or a `defer_tick`. For a strict body the
stages stabilize below their count, the diagonal is a fixpoint of the body, and two knots whose
bodies are graded-related are graded-related. -/

section Knots

variable {L : Type} {mem : L → Nat} {pacing : Pacing L mem}

/-- Stream stages of two strict bodies agree at every step below both stage counts. The
hypothesis `hAB` is strictness at lag `L` in the externals: agreement of the wires at lag
`L + 1` gives agreement of the outputs at lag `L`, at every horizon `≤ h`. -/
theorem stab_iterate_strict {n : Nat} {α : Type} {h L : Nat}
    {A A' : (Fin n → StepHist α) → (Fin n → StepHist α)}
    (hAB : ∀ h', h' ≤ h → ∀ (x y : Fin n → StepHist α),
      SAgreeL h' (L + 1) x y → SAgreeL h' L (A x) (A' y)) :
    ∀ (t m m' : Nat), t + L ≤ h → t < m → t < m' → ∀ i,
      ((iterate A (fun _ => StepHist.bot) m) i).view t
      = ((iterate A' (fun _ => StepHist.bot) m') i).view t := by
  intro t
  induction t using Nat.strongRecOn with
  | _ t ih =>
  intro m m' hth hm hm' i
  cases m with
  | zero => exact absurd hm (Nat.not_lt_zero _)
  | succ m =>
  cases m' with
  | zero => exact absurd hm' (Nat.not_lt_zero _)
  | succ m' =>
  show (A (iterate A _ m) i).view t = (A' (iterate A' _ m') i).view t
  exact hAB (t + L) hth _ _
    (fun i' t' ht' => ih t' (by omega) m m' (by omega) (by omega) (by omega) i')
    i t (by omega)

/-- `stab_iterate_strict` for tick bodies. -/
theorem stab_iterate_tick_strict {n : Nat} {σ : Type} {h L : Nat}
    {A A' : (Fin n → Nat → Trace σ) → (Fin n → Nat → Trace σ)}
    (hAB : ∀ h', h' ≤ h → ∀ (x y : Fin n → Nat → Trace σ),
      TAgreeL h' (L + 1) x y → TAgreeL h' L (A x) (A' y)) :
    ∀ (t m m' : Nat), t + L ≤ h → t < m → t < m' → ∀ i,
      (iterate A (fun _ _ => ([] : Trace σ)) m) i t
      = (iterate A' (fun _ _ => ([] : Trace σ)) m') i t := by
  intro t
  induction t using Nat.strongRecOn with
  | _ t ih =>
  intro m m' hth hm hm' i
  cases m with
  | zero => exact absurd hm (Nat.not_lt_zero _)
  | succ m =>
  cases m' with
  | zero => exact absurd hm' (Nat.not_lt_zero _)
  | succ m' =>
  show A (iterate A _ m) i t = A' (iterate A' _ m') i t
  exact hAB (t + L) hth _ _
    (fun i' t' ht' => ih t' (by omega) m m' (by omega) (by omega) (by omega) i')
    i t (by omega)

/-- Strictness of a stream body in its knot wire, externals fixed: agreement strictly below
a step gives agreement at it. -/
def SStrict {n : Nat} {α : Type} (A : (Fin n → StepHist α) → (Fin n → StepHist α)) : Prop :=
  ∀ h (x y : Fin n → StepHist α), SAgreeL h 1 x y → SAgree h (A x) (A y)

/-- `SStrict` for tick bodies. -/
def TStrict {n : Nat} {σ : Type} (A : (Fin n → Nat → Trace σ) → (Fin n → Nat → Trace σ)) : Prop :=
  ∀ h (x y : Fin n → Nat → Trace σ), TAgreeL h 1 x y → TAgree h (A x) (A y)

/-- `SStrict` in the graded form `stab_iterate_strict` takes, at lag `0`. -/
theorem SStrict.graded {n : Nat} {α : Type} {A : (Fin n → StepHist α) → (Fin n → StepHist α)}
    (hA : SStrict A) : ∀ h' (x y : Fin n → StepHist α), SAgreeL h' 1 x y → SAgreeL h' 0 (A x) (A y) :=
  fun h' x y hxy => SAgreeL_zero.mpr (hA h' x y hxy)

/-- `TStrict` in the graded form `stab_iterate_tick_strict` takes, at lag `0`. -/
theorem TStrict.graded {n : Nat} {σ : Type} {A : (Fin n → Nat → Trace σ) → (Fin n → Nat → Trace σ)}
    (hA : TStrict A) : ∀ h' (x y : Fin n → Nat → Trace σ), TAgreeL h' 1 x y → TAgreeL h' 0 (A x) (A y) :=
  fun h' x y hxy => FAgreeL_zero.mpr (hA h' x y hxy)

/-- `famFreeze` is an identity operator on a family that ascends by prefix at every step below `t`. -/
theorem famFreeze_eq_of_ascending' {m : Nat} {β : Type} [DecidableEq β] (f : Nat → Fin m → List β) :
    ∀ t, (∀ t', t' < t → ∀ i, f t' i <+: f (t' + 1) i) → famFreeze f t = f t
  | 0, _ => rfl
  | t + 1, hasc => by
    show (if (List.finRange m).all (fun i => ((famFreeze f t) i).isPrefixOf (f (t + 1) i))
      then f (t + 1) else famFreeze f t) = f (t + 1)
    rw [famFreeze_eq_of_ascending' f t (fun t' ht' i => hasc t' (Nat.lt_succ_of_lt ht') i)]
    rw [if_pos]
    exact List.all_eq_true.mpr (fun i _ => List.isPrefixOf_iff_prefix.mpr (hasc t (Nat.lt_succ_self t) i))

/-- The raw stream diagonal of a strict body is member-wise prefix-monotone: stage `t+1` and
stage `t+2` agree at step `t` (`stab_iterate_strict`), and stage `t+2` is a `StepHist`. -/
theorem strict_diag_mono {n : Nat} {α : Type} {A : (Fin n → StepHist α) → (Fin n → StepHist α)}
    (hA : SStrict A) (t : Nat) (i : Fin n) :
    ((iterate A (fun _ => StepHist.bot) (t + 1)) i).view t
      <+: ((iterate A (fun _ => StepHist.bot) (t + 2)) i).view (t + 1) := by
  rw [stab_iterate_strict (h := t) (L := 0) (fun h' _ x y hxy => hA.graded h' x y hxy) t (t + 1) (t + 2)
    (by omega) (by omega) (by omega) i]
  exact ((iterate A (fun _ => StepHist.bot) (t + 2)) i).mono t

/-- For a strict body the knot's view at `t` is the raw diagonal's: the freeze is transparent
by `strict_diag_mono`. -/
theorem fix_stream_view {ℓ : L} {α : Type} [DecidableEq α] {ord : StrOrd} {ret : Retries}
    {body : (SchedSem L mem pacing).Stream ℓ α ord ret → (SchedSem L mem pacing).Stream ℓ α ord ret}
    (hb : SStrict body) (i : Fin (mem ℓ)) (t : Nat) :
    (((SchedSem L mem pacing).fix_stream () body) i).view t
      = ((iterate body (fun _ => StepHist.bot) (t + 1)) i).view t := by
  show famFreeze (fun t j => ((iterate body (fun _ => StepHist.bot) (t + 1)) j).view t) t i = _
  rw [famFreeze_eq_of_ascending' (fun t j => ((iterate body (fun _ => StepHist.bot) (t + 1)) j).view t) t
    (fun t' _ j => strict_diag_mono hb t' j)]

/-- **The stream knot is its body's fixpoint**, step by step, for a body strict in the knot
wire. -/
theorem fix_stream_eq {ℓ : L} {α : Type} [DecidableEq α] {ord : StrOrd} {ret : Retries}
    {body : (SchedSem L mem pacing).Stream ℓ α ord ret → (SchedSem L mem pacing).Stream ℓ α ord ret}
    (hb : SStrict body) (i : Fin (mem ℓ)) (t : Nat) :
    (((SchedSem L mem pacing).fix_stream () body) i).view t
      = ((body ((SchedSem L mem pacing).fix_stream () body)) i).view t := by
  rw [fix_stream_view hb]
  show (body (iterate body (fun _ => StepHist.bot) t) i).view t = _
  refine hb t _ _ (fun j t' ht' => ?_) i t (Nat.le_refl _)
  rw [fix_stream_view hb]
  exact stab_iterate_strict (h := t') (L := 0) (fun h' _ x y hxy => hb.graded h' x y hxy) t' t (t' + 1)
    (by omega) (by omega) (by omega) j

/-- **The tick knot is its body's fixpoint**, step by step, for a body strict in the knot
wire. -/
theorem fix_tick_eq {ℓ : L} {σ : Type}
    {body : (SchedSem L mem pacing).Atomic ℓ σ → (SchedSem L mem pacing).Atomic ℓ σ}
    (hb : TStrict body) (i : Fin (mem ℓ)) (t : Nat) :
    ((SchedSem L mem pacing).fix_tick () body) i t
      = (body ((SchedSem L mem pacing).fix_tick () body)) i t := by
  rw [fix_tick_apply]
  show body (iterate body (fun _ _ => []) t) i t = _
  refine hb t _ _ (fun j t' ht' => ?_) i t (Nat.le_refl _)
  rw [fix_tick_apply]
  show (iterate body (fun _ _ => []) t) j t' = (iterate body (fun _ _ => []) (t' + 1)) j t'
  exact stab_iterate_tick_strict (h := t') (L := 0) (fun h' _ x y hxy => hb.graded h' x y hxy) t' t (t' + 1)
    (by omega) (by omega) (by omega) j

/-- Graded knot causality (stream): two knots whose bodies are strict and graded-related at
lag `Lg` in the wires they capture are graded-related at lag `Lg`. -/
theorem causalL_fix_stream {ℓ : L} {α : Type} [DecidableEq α] {ord : StrOrd} {ret : Retries}
    {h Lg : Nat} (d : Unit)
    {body body' : (SchedSem L mem pacing).Stream ℓ α ord ret → (SchedSem L mem pacing).Stream ℓ α ord ret}
    (hs : SStrict body) (hs' : SStrict body')
    (hb : ∀ h', h' ≤ h → ∀ (x y : (SchedSem L mem pacing).Stream ℓ α ord ret),
      SAgreeL h' (Lg + 1) x y → SAgreeL h' Lg (body x) (body' y)) :
    SAgreeL h Lg ((SchedSem L mem pacing).fix_stream d body)
      ((SchedSem L mem pacing).fix_stream d body') := by
  intro i t ht
  rw [fix_stream_view hs, fix_stream_view hs']
  exact stab_iterate_strict (h := h) (L := Lg) hb t (t + 1) (t + 1) ht (by omega) (by omega) i

/-- Graded knot causality (tick). No strictness hypothesis is needed: the tick knot is the
raw diagonal with no freeze. -/
theorem causalL_fix_tick {ℓ : L} {σ : Type} {h Lg : Nat} (d : Unit)
    {body body' : (SchedSem L mem pacing).Atomic ℓ σ → (SchedSem L mem pacing).Atomic ℓ σ}
    (hb : ∀ h', h' ≤ h → ∀ (x y : (SchedSem L mem pacing).Atomic ℓ σ),
      TAgreeL h' (Lg + 1) x y → TAgreeL h' Lg (body x) (body' y)) :
    TAgreeL h Lg ((SchedSem L mem pacing).fix_tick d body)
      ((SchedSem L mem pacing).fix_tick d body') := by
  intro i t ht
  rw [fix_tick_apply, fix_tick_apply]
  exact stab_iterate_tick_strict (h := h) (L := Lg) hb t (t + 1) (t + 1) ht (by omega) (by omega) i

/-! ### Base knot causality, without strictness

Two causally related bodies have agreeing Kleene stages at every count (induction on the
stage), hence agreeing diagonals and agreeing frozen diagonals. Strictness makes the
diagonal a fixpoint of the body (`fix_stream_eq`); plain causality already makes it a
function of the body's behaviour below the horizon. The graph-level `Node.denote_causal`
(`Graph/Causal.lean`) closes its knot case with these. -/

/-- Stages of two causally related stream bodies agree at every count. -/
theorem stab_iterate_causal {n : Nat} {α : Type} {h : Nat}
    {A A' : (Fin n → StepHist α) → (Fin n → StepHist α)}
    (hAB : ∀ h', h' ≤ h → ∀ (x y : Fin n → StepHist α), SAgree h' x y → SAgree h' (A x) (A' y)) :
    ∀ (m : Nat), SAgree h (iterate A (fun _ => StepHist.bot) m) (iterate A' (fun _ => StepHist.bot) m)
  | 0 => SAgree.refl h _
  | m + 1 => hAB h (Nat.le_refl _) _ _ (stab_iterate_causal hAB m)

/-- `stab_iterate_causal` for tick bodies. -/
theorem stab_iterate_tick_causal {n : Nat} {σ : Type} {h : Nat}
    {A A' : (Fin n → Nat → Trace σ) → (Fin n → Nat → Trace σ)}
    (hAB : ∀ h', h' ≤ h → ∀ (x y : Fin n → Nat → Trace σ), TAgree h' x y → TAgree h' (A x) (A' y)) :
    ∀ (m : Nat), TAgree h (iterate A (fun _ _ => ([] : Trace σ)) m) (iterate A' (fun _ _ => ([] : Trace σ)) m)
  | 0 => TAgree.refl h _
  | m + 1 => hAB h (Nat.le_refl _) _ _ (stab_iterate_tick_causal hAB m)

/-- Knot causality (stream): two knots whose bodies are causally related below the horizon
agree below it. -/
theorem causal_fix_stream {ℓ : L} {α : Type} [DecidableEq α] {ord : StrOrd} {ret : Retries}
    {h : Nat} (d : Unit)
    {body body' : (SchedSem L mem pacing).Stream ℓ α ord ret → (SchedSem L mem pacing).Stream ℓ α ord ret}
    (hb : ∀ h', h' ≤ h → ∀ (x y : (SchedSem L mem pacing).Stream ℓ α ord ret),
      SAgree h' x y → SAgree h' (body x) (body' y)) :
    SAgree h ((SchedSem L mem pacing).fix_stream d body)
      ((SchedSem L mem pacing).fix_stream d body') := by
  intro i t ht
  show famFreeze (fun t j => ((iterate body (fun _ => StepHist.bot) (t + 1)) j).view t) t i
    = famFreeze (fun t j => ((iterate body' (fun _ => StepHist.bot) (t + 1)) j).view t) t i
  exact congrFun (famFreeze_congr_le t (fun t' ht' => funext fun j =>
    stab_iterate_causal (h := h) hb (t' + 1) j t' (ht'.trans ht))) i

/-- Knot causality (tick). -/
theorem causal_fix_tick {ℓ : L} {σ : Type} {h : Nat} (d : Unit)
    {body body' : (SchedSem L mem pacing).Atomic ℓ σ → (SchedSem L mem pacing).Atomic ℓ σ}
    (hb : ∀ h', h' ≤ h → ∀ (x y : (SchedSem L mem pacing).Atomic ℓ σ),
      TAgree h' x y → TAgree h' (body x) (body' y)) :
    TAgree h ((SchedSem L mem pacing).fix_tick d body)
      ((SchedSem L mem pacing).fix_tick d body') := by
  intro i t ht
  rw [fix_tick_apply, fix_tick_apply]
  exact stab_iterate_tick_causal (h := h) hb (t + 1) i t ht

end Knots

/-! ## Graded congruences for the remaining ops

The base `causal_*` facts lifted to any lag by `lift*`: an immediate op keeps its input's
lag, a two-input op takes the `max`. They are stated with wire variables so the lifted
functional is a higher-order pattern; the graph theorem `Dag.denote_agreeL`
(`Graph/Causal.lean`) applies them to `env.get` terms. -/

section GradedExtra

variable {L : Type} {mem : L → Nat} {pacing : Pacing L mem}

theorem causalL_values {ℓ : L} {K V : Type} [DecidableEq K] [DecidableEq V] {ord : StrOrd} {ret : Retries} {h n : Nat}
    {k k' : (SchedSem L mem pacing).KeyedStream ℓ K V ord ret} (hk : SAgreeL h n k k') :
    SAgreeL h n ((SchedSem L mem pacing).values k) ((SchedSem L mem pacing).values k') :=
  liftSS (fun _ _ _ hx => causal_values hx) hk

theorem causalL_into_keyed {ℓ : L} {K V : Type} [DecidableEq K] [DecidableEq V] {ord : StrOrd} {h n : Nat}
    {s s' : (SchedSem L mem pacing).Stream ℓ (K × V) ord .exactlyOnce} (hs : SAgreeL h n s s') :
    SAgreeL h n ((SchedSem L mem pacing).into_keyed s) ((SchedSem L mem pacing).into_keyed s') := hs

theorem causalL_kassume_ordering {ℓ : L} {K V : Type} [DecidableEq K] [DecidableEq V] {h n : Nat}
    {u u' : (SchedSem L mem pacing).KeyedStream ℓ K V .noOrder .exactlyOnce} (d : Unit) (hu : SAgreeL h n u u') :
    SAgreeL h n ((SchedSem L mem pacing).kassume_ordering u d) ((SchedSem L mem pacing).kassume_ordering u' d) := hu

theorem causalL_kfold_early_stop {ℓ : L} {K V A : Type} [DecidableEq K] [DecidableEq V] [DecidableEq A] {h n : Nat}
    {ks ks' : (SchedSem L mem pacing).KeyedStream ℓ K V .totalOrder .exactlyOnce}
    (f : A → V → A × Bool) (init : A) (hk : SAgreeL h n ks ks') :
    FAgreeL h n ((SchedSem L mem pacing).kfold_early_stop ks f init)
      ((SchedSem L mem pacing).kfold_early_stop ks' f init) :=
  liftSF (fun _ _ _ hx => causal_kfold_early_stop f init hx) hk

theorem causalL_kget_max_key {ℓ : L} {α K V : Type} [DecidableEq α] [DecidableEq K] [DecidableEq V] [LinearOrder K]
    {ord : StrOrd} {ret : Retries} {h n : Nat}
    {ks ks' : (SchedSem L mem pacing).KeyedSingleton ℓ α K V ord ret .boundedValue} (hk : FAgreeL h n ks ks') :
    FAgreeL h n ((SchedSem L mem pacing).kget_max_key ks) ((SchedSem L mem pacing).kget_max_key ks') :=
  liftFF (fun _ _ _ hx => causal_kget_max_key hx) hk

theorem causalL_weaken_retries {ℓ : L} {α : Type} [DecidableEq α] {ord : StrOrd} {h n : Nat}
    {s s' : (SchedSem L mem pacing).Stream ℓ α ord .exactlyOnce} (hs : SAgreeL h n s s') :
    SAgreeL h n ((SchedSem L mem pacing).weaken_retries s) ((SchedSem L mem pacing).weaken_retries s') := hs

theorem causalL_union {ℓ : L} {α : Type} [DecidableEq α] {ret : Retries} {h n n' : Nat}
    {a a' b b' : (SchedSem L mem pacing).Stream ℓ α .noOrder ret}
    (ha : SAgreeL h n a a') (hb : SAgreeL h n' b b') :
    SAgreeL h (max n n') ((SchedSem L mem pacing).union a b) ((SchedSem L mem pacing).union a' b') :=
  liftSS2 (fun _ _ _ _ _ hx hy => causal_union hx hy) ha hb

theorem causalL_assume_ordering {ℓ : L} {α : Type} [DecidableEq α] {h n : Nat}
    {u u' : (SchedSem L mem pacing).Stream ℓ α .noOrder .exactlyOnce} (d : Unit) (hu : SAgreeL h n u u') :
    SAgreeL h n ((SchedSem L mem pacing).assume_ordering u d) ((SchedSem L mem pacing).assume_ordering u' d) := hu

theorem causalL_fold {ℓ : L} {α σ : Type} [DecidableEq α] {ord : StrOrd} {ret : Retries} {h n : Nat}
    (g : σ → α → σ) (init : σ) (Pre : α → α → Prop) [DecidableRel Pre] (ok : FoldOk ord ret Pre g)
    {s s' : (SchedSem L mem pacing).Stream ℓ α ord ret} (hs : SAgreeL h n s s') :
    FAgreeL h n ((SchedSem L mem pacing).fold g init Pre ok s) ((SchedSem L mem pacing).fold g init Pre ok s') :=
  liftSF (fun _ _ _ hx => causal_fold g init Pre ok hx) hs

theorem causalL_fold_monotone {ℓ : L} {α σ : Type} [DecidableEq α] {ord : StrOrd} {ret : Retries} {h n : Nat}
    (vo : ValueOrder σ) (g : σ → α → σ) (init : σ) (ok : FoldOk ord ret (fun _ _ => True) g)
    (hinfl : ∀ s x, vo.le s (g s x))
    {s s' : (SchedSem L mem pacing).Stream ℓ α ord ret} (hs : SAgreeL h n s s') :
    FAgreeL h n ((SchedSem L mem pacing).fold_monotone vo g init ok hinfl s)
      ((SchedSem L mem pacing).fold_monotone vo g init ok hinfl s') :=
  liftSF (fun _ _ _ hx => causal_fold_monotone vo g init ok hinfl hx) hs

theorem causalL_snapshot {ℓ : L} {α σ : Type} [DecidableEq α] {ord : StrOrd} {ret : Retries} {b : SingBound σ} {h n : Nat}
    {s s' : (SchedSem L mem pacing).Singleton ℓ α σ ord ret b} (d : Unit) (hs : FAgreeL h n s s') :
    TAgreeL h n ((SchedSem L mem pacing).snapshot s d) ((SchedSem L mem pacing).snapshot s' d) :=
  liftFF (fun _ _ _ hx => causal_snapshot d hx) hs

theorem causalL_batch {ℓ : L} {α : Type} [DecidableEq α] {h n : Nat} {ord : StrOrd} {ret : Retries}
    {s s' : (SchedSem L mem pacing).Stream ℓ α ord ret} (d : Unit) (hs : SAgreeL h n s s') :
    TAgreeL h n ((SchedSem L mem pacing).batch s d) ((SchedSem L mem pacing).batch s' d) :=
  liftSF (fun _ _ _ hx => causal_batch d hx) hs

theorem causalL_mapTick {ℓ : L} {α β : Type} {h n : Nat}
    {t t' : (SchedSem L mem pacing).Atomic ℓ α} (f : Fin (mem ℓ) → α → β) (ht : TAgreeL h n t t') :
    TAgreeL h n ((SchedSem L mem pacing).mapTick t f) ((SchedSem L mem pacing).mapTick t' f) :=
  liftFF (fun _ _ _ hx => causal_mapTick f hx) ht

theorem causalL_zipTick {ℓ : L} {α β : Type} {h n n' : Nat}
    {a a' : (SchedSem L mem pacing).Atomic ℓ α} {b b' : (SchedSem L mem pacing).Atomic ℓ β}
    (ha : TAgreeL h n a a') (hb : TAgreeL h n' b b') :
    TAgreeL h (max n n') ((SchedSem L mem pacing).zipTick a b) ((SchedSem L mem pacing).zipTick a' b') :=
  liftFF2 (fun _ _ _ _ _ hx hy => causal_zipTick hx hy) ha hb

theorem causalL_flattenOrdered {ℓ : L} {β : Type} [DecidableEq β] {h n : Nat}
    {t t' : (SchedSem L mem pacing).Atomic ℓ (List β)} (ht : TAgreeL h n t t') :
    TAgreeL h n ((SchedSem L mem pacing).flattenOrdered t) ((SchedSem L mem pacing).flattenOrdered t') :=
  liftFF (fun _ _ _ hx => causal_flattenOrdered hx) ht

theorem causalL_flattenUnordered {ℓ : L} {β : Type} [DecidableEq β] {h n : Nat}
    {t t' : (SchedSem L mem pacing).Atomic ℓ (List β)} (ht : TAgreeL h n t t') :
    TAgreeL h n ((SchedSem L mem pacing).flattenUnordered t) ((SchedSem L mem pacing).flattenUnordered t') :=
  liftFF (fun _ _ _ hx => causal_flattenUnordered hx) ht

theorem causalL_firstTick {ℓ : L} {α : Type} [DecidableEq α] {ret : Retries} {h n : Nat}
    {t t' : (SchedSem L mem pacing).AtomicStream ℓ α .totalOrder ret} (ht : TAgreeL h n t t') :
    TAgreeL h n ((SchedSem L mem pacing).firstTick t) ((SchedSem L mem pacing).firstTick t') :=
  liftFF (fun _ _ _ hx => causal_firstTick hx) ht

theorem causalL_allTicks {ℓ : L} {β : Type} [DecidableEq β] {ord : StrOrd} {h n : Nat}
    {bs bs' : (SchedSem L mem pacing).AtomicStream ℓ β ord .exactlyOnce} (hb : TAgreeL h n bs bs') :
    SAgreeL h n ((SchedSem L mem pacing).allTicks bs) ((SchedSem L mem pacing).allTicks bs') :=
  liftFS (fun _ _ _ hx => causal_allTicks hx) hb

theorem causalL_tinto_keyed {ℓ : L} {K V : Type} [DecidableEq K] [DecidableEq V] {ord : StrOrd} {h n : Nat}
    {x x' : (SchedSem L mem pacing).AtomicStream ℓ (K × V) ord .exactlyOnce} (hx : TAgreeL h n x x') :
    TAgreeL h n ((SchedSem L mem pacing).tinto_keyed x) ((SchedSem L mem pacing).tinto_keyed x') := hx

theorem causalL_tassume_ordering {ℓ : L} {α : Type} [DecidableEq α] {h n : Nat}
    {u u' : (SchedSem L mem pacing).AtomicStream ℓ α .noOrder .exactlyOnce} (d : Unit) (hu : TAgreeL h n u u') :
    TAgreeL h n ((SchedSem L mem pacing).tassume_ordering u d) ((SchedSem L mem pacing).tassume_ordering u' d) := hu

theorem causalL_reduce_watermark {ℓ : L} {K V : Type} [DecidableEq K] [Hashable K] [DecidableEq V]
    [LinearOrder K] {ord : StrOrd} {h n n' : Nat}
    {ks ks' : (SchedSem L mem pacing).AtomicKeyedStream ℓ K V ord .exactlyOnce}
    {wm wm' : (SchedSem L mem pacing).Atomic ℓ (Option K)}
    (f : V → V → V) (Pre : V → V → Prop) [DecidableRel Pre] (ok : FoldOk ord .exactlyOnce Pre (reduceStep f))
    (hk : TAgreeL h n ks ks') (hw : TAgreeL h n' wm wm') :
    TAgreeL h (max n n') ((SchedSem L mem pacing).reduce_watermark ks wm f Pre ok)
      ((SchedSem L mem pacing).reduce_watermark ks' wm' f Pre ok) :=
  liftFF2 (fun _ _ _ _ _ hx hy => causal_reduce_watermark f Pre ok hx hy) hk hw

theorem causalL_kentries {ℓ : L} {K V : Type} [DecidableEq K] [DecidableEq V] {ord : StrOrd} {h n : Nat}
    {ks ks' : (SchedSem L mem pacing).KeyedStream ℓ K V ord .exactlyOnce} (hk : SAgreeL h n ks ks') :
    SAgreeL h n ((SchedSem L mem pacing).kentries ks) ((SchedSem L mem pacing).kentries ks') := hk

theorem causalL_kmap {ℓ : L} {K V W : Type} [DecidableEq K] [DecidableEq V] [DecidableEq W] {ord : StrOrd} {h n : Nat}
    {ks ks' : (SchedSem L mem pacing).KeyedStream ℓ K V ord .exactlyOnce} (f : Fin (mem ℓ) → V → W)
    (hk : SAgreeL h n ks ks') :
    SAgreeL h n ((SchedSem L mem pacing).kmap ks f) ((SchedSem L mem pacing).kmap ks' f) :=
  liftSS (fun _ _ _ hx => causal_kmap f hx) hk

theorem causalL_kallTicks {ℓ : L} {K V : Type} [DecidableEq K] [DecidableEq V] {ord : StrOrd} {h n : Nat}
    {bs bs' : (SchedSem L mem pacing).AtomicKeyedStream ℓ K V ord .exactlyOnce} (hb : TAgreeL h n bs bs') :
    SAgreeL h n ((SchedSem L mem pacing).kallTicks bs) ((SchedSem L mem pacing).kallTicks bs') :=
  liftFS (fun _ _ _ hx => causal_kallTicks hx) hb

theorem causalL_kfold {ℓ : L} {K V A : Type} [DecidableEq K] [Hashable K] [DecidableEq V] [DecidableEq A]
    {ord : StrOrd} {h n : Nat} (g : A → V → A) (init : A) (Pre : V → V → Prop) [DecidableRel Pre]
    (ok : FoldOk ord .exactlyOnce Pre g)
    {ks ks' : (SchedSem L mem pacing).KeyedStream ℓ K V ord .exactlyOnce} (hk : SAgreeL h n ks ks') :
    FAgreeL h n ((SchedSem L mem pacing).kfold g init Pre ok ks) ((SchedSem L mem pacing).kfold g init Pre ok ks') :=
  liftSF (fun _ _ _ hx => causal_kfold g init Pre ok hx) hk

theorem causalL_ksnapshot {ℓ : L} {α K V : Type} [DecidableEq α] [DecidableEq K] [DecidableEq V]
    {ord : StrOrd} {ret : Retries} {b : KSBound} {h n : Nat}
    {ks ks' : (SchedSem L mem pacing).KeyedSingleton ℓ α K V ord ret b} (d : Unit) (hk : FAgreeL h n ks ks') :
    TAgreeL h n ((SchedSem L mem pacing).ksnapshot ks d) ((SchedSem L mem pacing).ksnapshot ks' d) :=
  liftFF (fun _ _ _ hx => causal_ksnapshot d hx) hk

theorem causalL_tfold {ℓ : L} {α σ : Type} [DecidableEq α] {ord : StrOrd} {ret : Retries} {h n : Nat}
    (g : σ → α → σ) (init : σ) (Pre : α → α → Prop) [DecidableRel Pre] (ok : FoldOk ord ret Pre g)
    {bs bs' : (SchedSem L mem pacing).AtomicStream ℓ α ord ret} (hb : TAgreeL h n bs bs') :
    TAgreeL h n ((SchedSem L mem pacing).tfold g init Pre ok bs) ((SchedSem L mem pacing).tfold g init Pre ok bs') :=
  liftFF (fun _ _ _ hx => causal_tfold g init Pre ok hx) hb

theorem causalL_latest {ℓ : L} {σ : Type} [DecidableEq σ] {h n : Nat}
    {t t' : (SchedSem L mem pacing).Atomic ℓ (Option σ)} (ht : TAgreeL h n t t') :
    FAgreeL h n ((SchedSem L mem pacing).latest t) ((SchedSem L mem pacing).latest t') :=
  liftFF (fun _ _ _ hx => causal_latest hx) ht

theorem causalL_latest_monotone {ℓ : L} {h n : Nat}
    {t t' : (SchedSem L mem pacing).Atomic ℓ (Option Nat)} (ht : TAgreeL h n t t') :
    FAgreeL h n ((SchedSem L mem pacing).latest_monotone t) ((SchedSem L mem pacing).latest_monotone t') :=
  liftFF (fun _ _ _ hx => causal_latest_monotone hx) ht

/-- Leafwise agreement at lag `n` of two shaped wire tuples. -/
@[reducible] def TAgreeOfL {ℓ : L} (h n : Nat) : (sh : TickShape) →
    AtomicOf (SchedTick.Tk mem) (SchedTick.TS mem) (SchedTick.TKS mem) (SchedTick.TKStr mem) ℓ sh →
    AtomicOf (SchedTick.Tk mem) (SchedTick.TS mem) (SchedTick.TKS mem) (SchedTick.TKStr mem) ℓ sh → Prop
  | .sing _, x, y => TAgreeL h n x y
  | .stream _ _ _ _, x, y => TAgreeL h n x y
  | .ksing _ _ _ _, x, y => TAgreeL h n x y
  | .kstream _ _ _ _ _ _, x, y => TAgreeL h n x y
  | .pair a b, x, y => TAgreeOfL h n a x.1 y.1 ∧ TAgreeOfL h n b x.2 y.2

/-- `TAgreeOfL` at lag `n` is `TAgreeOf` at horizon `h - n`, leafwise. -/
theorem TAgreeOfL.to_base {ℓ : L} {h n : Nat} (hn : n ≤ h) : (sh : TickShape) →
    {x y : AtomicOf (SchedTick.Tk mem) (SchedTick.TS mem) (SchedTick.TKS mem) (SchedTick.TKStr mem) ℓ sh} →
    TAgreeOfL h n sh x y → TAgreeOf (h - n) sh x y
  | .sing _, _, _, hxy => FAgreeL.to_base hn hxy
  | .stream _ _ _ _, _, _, hxy => FAgreeL.to_base hn hxy
  | .ksing _ _ _ _, _, _, hxy => FAgreeL.to_base hn hxy
  | .kstream _ _ _ _ _ _, _, _, hxy => FAgreeL.to_base hn hxy
  | .pair a b, _, _, hxy => ⟨TAgreeOfL.to_base hn a hxy.1, TAgreeOfL.to_base hn b hxy.2⟩

/-- The converse of `TAgreeOfL.to_base`. -/
theorem TAgreeOfL.of_base {ℓ : L} {h n : Nat} (hn : n ≤ h) : (sh : TickShape) →
    {x y : AtomicOf (SchedTick.Tk mem) (SchedTick.TS mem) (SchedTick.TKS mem) (SchedTick.TKStr mem) ℓ sh} →
    TAgreeOf (h - n) sh x y → TAgreeOfL h n sh x y
  | .sing _, _, _, hxy => FAgreeL.of_base hn hxy
  | .stream _ _ _ _, _, _, hxy => FAgreeL.of_base hn hxy
  | .ksing _ _ _ _, _, _, hxy => FAgreeL.of_base hn hxy
  | .kstream _ _ _ _ _ _, _, _, hxy => FAgreeL.of_base hn hxy
  | .pair a b, _, _, hxy => ⟨TAgreeOfL.of_base hn a hxy.1, TAgreeOfL.of_base hn b hxy.2⟩

/-- Above the horizon every graded agreement is vacuous. -/
theorem TAgreeOfL.vacuous {ℓ : L} {h n : Nat} (hn : h < n) : (sh : TickShape) →
    (x y : AtomicOf (SchedTick.Tk mem) (SchedTick.TS mem) (SchedTick.TKS mem) (SchedTick.TKStr mem) ℓ sh) →
    TAgreeOfL h n sh x y
  | .sing _, _, _ => fun _ _ ht => absurd ht (by omega)
  | .stream _ _ _ _, _, _ => fun _ _ ht => absurd ht (by omega)
  | .ksing _ _ _ _, _, _ => fun _ _ ht => absurd ht (by omega)
  | .kstream _ _ _ _ _ _, _, _ => fun _ _ ht => absurd ht (by omega)
  | .pair a b, x, y => ⟨TAgreeOfL.vacuous hn a x.1 y.1, TAgreeOfL.vacuous hn b x.2 y.2⟩

/-- Graded causality of the tick former, by cases on whether the lag exceeds the horizon. -/
theorem causalL_tick_scan {ℓ : L} {h n : Nat} (sts ins outs : TickShape)
    {x x' : AtomicOf (SchedTick.Tk mem) (SchedTick.TS mem) (SchedTick.TKS mem) (SchedTick.TKStr mem) ℓ ins}
    (g : Fin (mem ℓ) → BoundedOf SchedTick.BS SchedTick.BStr SchedTick.BKS SchedTick.BKStr sts →
      BoundedOf SchedTick.BS SchedTick.BStr SchedTick.BKS SchedTick.BKStr ins →
      BoundedOf SchedTick.BS SchedTick.BStr SchedTick.BKS SchedTick.BKStr sts × BoundedOf SchedTick.BS SchedTick.BStr SchedTick.BKS SchedTick.BKStr outs)
    (init : SeedOf sts) (hx : TAgreeOfL h n ins x x') :
    TAgreeOfL h n outs ((SchedSem L mem pacing).tick_scan sts ins outs x g init)
      ((SchedSem L mem pacing).tick_scan sts ins outs x' g init) := by
  rcases Nat.lt_or_ge h n with hlt | hle
  · exact TAgreeOfL.vacuous hlt outs _ _
  · exact TAgreeOfL.of_base hle outs (causal_tick_scan sts ins outs g init (TAgreeOfL.to_base hle ins hx))

end GradedExtra

end Hydro
