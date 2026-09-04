import Hydro.Sched

/-!
# Machine-op causality (`SchedCausal`)

**Causality**: a machine op's output views at steps `≤ h` are a
function of its inputs' views at steps `≤ h`. This file proves it for
every `SchedSem` op — one congruence lemma per operator over the
view-agreement relations below (totality over the signature is the
cheat detector: an op without a congruence lemma could smuggle
acausal information) — and derives the two knot-stabilization roots:

- **stage stabilization**: the shift-iterates of a causal body are
  view-stable below their index (`stabIterate` / `stabIterateTick`),
  so every stage of a knot agrees with every later stage — and with
  the diagonal — at the views its own δ atoms read;
- **fix causality**: a knot's diagonal is itself causal in the wires
  its body captures (`causal_fix_stream`/`causal_fix_tick`), so
  stabilization propagates through nested knots.

The lemmas are *heterogeneous* (two different op-built composites,
same shape, different leaf wires): the same statements serve both
"one run at two stages" and "stage body vs. diagonal body" uses.
Decisions are identical on both sides — causality is about wires;
schedules never diverge between the two runs being compared.

These roots are what the coupling corner's knot `wf` triple
(`Couple.lean`, generated per knot by `hydro def`) stands on; they
closed `SCHED_AUDIT.md` F1 (the square-era `hsat` premise, D37/D42).
-/

namespace Hydro

/-! ## View-agreement relations (one per machine carrier shape) -/

/-- Two step wires agree at all views up to `h`. -/
def VAgree {α : Type} (h : Nat) (x y : StepHist α) : Prop :=
  ∀ t, t ≤ h → x.view t = y.view t

/-- Stream carriers: member-wise view agreement up to `h`. -/
def SAgree {n : Nat} {α : Type} (h : Nat)
    (x y : Fin n → StepHist α) : Prop :=
  ∀ i, VAgree h (x i) (y i)

/-- Fold carriers (the live value per step): value agreement at all
steps up to `h`. -/
def FAgree {n : Nat} {σ : Type} (h : Nat)
    (x y : Fin n → Nat → σ) : Prop :=
  ∀ i, ∀ t, t ≤ h → x i t = y i t

/-- Atomic carriers (`Atomic`/`AtomicStream`): entry
agreement at all steps up to `h`. -/
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

/-! Downward monotonicity: agreement at a horizon holds below it. -/

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

/-! ## Helper congruences over the machine primitives -/

/-- Increments agree strictly below the agreement horizon. -/
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

/-- Members of the tick skeleton are bounded by its horizon. -/
theorem tickSteps_mem_le {p : Nat → Bool} {t st : Nat}
    (hst : st ∈ tickSteps p t) : st ≤ t := by
  have := (List.mem_filter.mp hst).1
  exact Nat.lt_succ_iff.mp (List.mem_range.mp this)

/-- `batchesFrom` congruence: equal sources at every walked step. -/
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

/-! ## Per-op causality (stream and keyed ops) -/

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

/-- A delivered, sender-tagged wire agrees below the horizon when the
sender's wire does. -/
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

/-! ## Per-op causality (tick-boundary and in-tick ops) -/

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
  show batchesFrom ((s i).view) (tickSteps (pacing.ticks ℓ i) t) 0
    = batchesFrom ((s' i).view) (tickSteps (pacing.ticks ℓ i) t) 0
  exact batchesFrom_congr _
    (fun st hst => hs i st ((tickSteps_mem_le hst).trans ht)) 0

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
  show (init :: t i u).take (deferReleased (pacing.ticks ℓ i) (fun k => (t i k).length) u)
    = (init :: t' i u).take (deferReleased (pacing.ticks ℓ i) (fun k => (t' i k).length) u)
  rw [ht i u hu, deferReleased_congr _ u (fun v hv => by rw [ht i v (Nat.le_trans hv hu)])]

theorem causal_flattenOrdered {ℓ : L} {β : Type} [DecidableEq β] {h : Nat}
    {t t' : (SchedSem L mem pacing).Atomic ℓ (List β)}
    (ht : TAgree h t t') :
    TAgree h ((SchedSem L mem pacing).flattenOrdered t)
      ((SchedSem L mem pacing).flattenOrdered t') := ht

theorem causal_flattenUnordered {ℓ : L} {β : Type} [DecidableEq β]
    {h : Nat}
    {t t' : (SchedSem L mem pacing).Atomic ℓ (List β)}
    (ht : TAgree h t t') :
    TAgree h ((SchedSem L mem pacing).flattenUnordered t)
      ((SchedSem L mem pacing).flattenUnordered t') := ht

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
  show (t i k).map List.head? = (t' i k).map List.head?
  rw [ht i k hk]

/-! ### The tick former: a tick's output depends on this step's inputs
only (leafwise agreement in, leafwise agreement out) -/

/-- Leafwise step-`h` agreement of two shaped wire tuples. -/
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

/-! ## Per-op causality (`famHist` producers and knots) -/

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
  show famFreeze (fun step j => (bs j step).flatten) u i
    = famFreeze (fun step j => (bs' j step).flatten) u i
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
  show scanAcrossTicksTrace (reduceWatermarkEager f) (SchedKMap.empty, none) (Trace.zip (ks i t) (wm i t))
    = scanAcrossTicksTrace (reduceWatermarkEager f) (SchedKMap.empty, none) (Trace.zip (ks' i t) (wm' i t))
  rw [hk i t ht, hw i t ht]

/-! D81: the bench client's vocabulary. -/

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
  show famFreeze (fun step j => (bs j step).flatten) u i
    = famFreeze (fun step j => (bs' j step).flatten) u i
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

/-- Shift-iterates of two causally-related step functions agree below
the horizon (both sides at the same iterate count). -/
theorem iterate_shift_congr {n : Nat} {α : Type} {h : Nat}
    {A B : (Fin n → StepHist α) → (Fin n → StepHist α)} (p : Fin n → Nat → Bool)
    (hAB : ∀ h', h' ≤ h → ∀ (x y : Fin n → StepHist α), SAgree h' x y →
      SAgree h' (A x) (B y)) :
    ∀ m, SAgree h
      (iterate (fun x j => ((A x) j).shift (p j)) (fun _ => StepHist.bot) m)
      (iterate (fun x j => ((B x) j).shift (p j)) (fun _ => StepHist.bot) m)
  | 0 => SAgree.refl h _
  | m + 1 => by
    intro i t ht
    show gateView (p i) (A (iterate _ _ m) i).view t = gateView (p i) (B (iterate _ _ m) i).view t
    exact gateView_congr (p i) _ _ h
      (fun t' ht' => hAB h (Nat.le_refl _) _ _ (iterate_shift_congr p hAB m) i t' (Nat.le_of_lt ht')) t ht

/-- Knot causality (stream): the diagonal of a knot is causal in the
wires its body captures — stated as body-level heterogeneity. -/
theorem causal_fix_stream {ℓ : L} {α : Type} [DecidableEq α]
    {ord : StrOrd} {ret : Retries} {h : Nat} (d : Unit)
    {body body' : (SchedSem L mem pacing).Stream ℓ α ord ret →
      (SchedSem L mem pacing).Stream ℓ α ord ret}
    (hb : ∀ h', h' ≤ h →
      ∀ (x y : (SchedSem L mem pacing).Stream ℓ α ord ret),
      SAgree h' x y → SAgree h' (body x) (body' y)) :
    SAgree h ((SchedSem L mem pacing).fix_stream d body)
      ((SchedSem L mem pacing).fix_stream d body') := by
  intro i u hu
  show famFreeze (fun t j =>
      ((iterate (fun x j' => ((body x) j').shift (pacing.active ℓ j'))
        (fun _ => StepHist.bot) (t + 1)) j).view t) u i
    = famFreeze (fun t j =>
      ((iterate (fun x j' => ((body' x) j').shift (pacing.active ℓ j'))
        (fun _ => StepHist.bot) (t + 1)) j).view t) u i
  exact congrFun (famFreeze_congr_le u (fun u' hu' => funext fun j =>
    iterate_shift_congr (fun j' => pacing.active ℓ j')
      (fun h'' hh'' x y hxy => hb h'' (hh''.trans (hu'.trans hu)) x y hxy)
      (u' + 1) j u' (Nat.le_refl _))) i

/-- Tick-iterates of two causally-related step functions agree below
the horizon. -/
theorem iterate_tickshift_congr {n : Nat} {σ : Type} {h : Nat}
    {A B : (Fin n → Nat → Trace σ) → (Fin n → Nat → Trace σ)} (p : Fin n → Nat → Bool)
    (hAB : ∀ h', h' ≤ h → ∀ (x y : Fin n → Nat → Trace σ),
      TAgree h' x y → TAgree h' (A x) (B y)) :
    ∀ m, TAgree h
      (iterate (fun x j => gateView (p j) ((A x) j)) (fun _ _ => []) m)
      (iterate (fun x j => gateView (p j) ((B x) j)) (fun _ _ => []) m)
  | 0 => TAgree.refl h _
  | m + 1 => by
    intro i t ht
    show gateView (p i) ((A (iterate _ _ m)) i) t = gateView (p i) ((B (iterate _ _ m)) i) t
    exact gateView_congr (p i) _ _ h
      (fun t' ht' => hAB h (Nat.le_refl _) _ _ (iterate_tickshift_congr p hAB m) i t' (Nat.le_of_lt ht')) t ht

/-- Knot causality (tick). -/
theorem causal_fix_tick {ℓ : L} {σ : Type} {h : Nat} (d : Unit)
    {body body' : (SchedSem L mem pacing).Atomic ℓ σ →
      (SchedSem L mem pacing).Atomic ℓ σ}
    (hb : ∀ h', h' ≤ h →
      ∀ (x y : (SchedSem L mem pacing).Atomic ℓ σ),
      TAgree h' x y → TAgree h' (body x) (body' y)) :
    TAgree h ((SchedSem L mem pacing).fix_tick d body)
      ((SchedSem L mem pacing).fix_tick d body') := by
  intro i u hu
  show (iterate (fun x j => gateView (pacing.active ℓ j) ((body x) j)) (fun _ _ => []) (u + 1)) i u
    = (iterate (fun x j => gateView (pacing.active ℓ j) ((body' x) j)) (fun _ _ => []) (u + 1)) i u
  exact iterate_tickshift_congr (fun j => pacing.active ℓ j)
    (fun h'' hh'' x y hxy => hb h'' (hh''.trans hu) x y hxy)
    (u + 1) i u (Nat.le_refl _)

end KnotOps

/-! ## Stage-stabilization roots

Shift-iterates of (hetero-)causal step functions are view-stable below
their iterate count: every stage of a knot shows the same views — at
the steps its own δ atoms read — as every later stage. These are the
roots the corner's knot causality (`co_fix_view`, `causal_fix_*`)
stands on. -/

/-- Stream root: views below both iterate counts agree. -/
theorem stab_iterate_shift {n : Nat} {α : Type} {h : Nat}
    {A A' : (Fin n → StepHist α) → (Fin n → StepHist α)} (p : Fin n → Nat → Bool)
    (hAB : ∀ h', h' ≤ h → ∀ (x y : Fin n → StepHist α),
      SAgree h' x y → SAgree h' (A x) (A' y)) :
    ∀ (t m m' : Nat), t ≤ h → t < m → t < m' → ∀ i,
      ((iterate (fun x j => ((A x) j).shift (p j))
        (fun _ => StepHist.bot) m) i).view t
      = ((iterate (fun x j => ((A' x) j).shift (p j))
        (fun _ => StepHist.bot) m') i).view t := by
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
  cases t with
  | zero => rfl
  | succ t =>
    show gateView (p i) (A (iterate _ _ m) i).view (t + 1)
      = gateView (p i) (A' (iterate _ _ m') i).view (t + 1)
    rw [gateView_succ, gateView_succ]
    split
    · exact hAB t (Nat.le_of_succ_le hth) _ _
        (fun i' t' ht' => ih t' (Nat.lt_succ_of_le ht') m m'
          (ht'.trans (Nat.le_of_succ_le hth))
          (Nat.lt_of_le_of_lt ht' (Nat.lt_of_succ_lt_succ hm))
          (Nat.lt_of_le_of_lt ht' (Nat.lt_of_succ_lt_succ hm')) i')
        i t (Nat.le_refl _)
    · exact ih t (Nat.lt_succ_self t) (m + 1) (m' + 1) (Nat.le_of_succ_le hth)
        (Nat.lt_of_succ_lt hm) (Nat.lt_of_succ_lt hm') i

/-- Tick root: entries below both iterate counts agree. -/
theorem stab_iterate_tick {n : Nat} {σ : Type} {h : Nat}
    {A A' : (Fin n → Nat → Trace σ) → (Fin n → Nat → Trace σ)} (p : Fin n → Nat → Bool)
    (hAB : ∀ h', h' ≤ h → ∀ (x y : Fin n → Nat → Trace σ),
      TAgree h' x y → TAgree h' (A x) (A' y)) :
    ∀ (t m m' : Nat), t ≤ h → t < m → t < m' → ∀ i,
      (iterate (τ := Fin n → Nat → Trace σ)
        (fun x j => gateView (p j) ((A x) j)) (fun _ _ => ([] : Trace σ)) m) i t
      = (iterate (τ := Fin n → Nat → Trace σ)
        (fun x j => gateView (p j) ((A' x) j)) (fun _ _ => ([] : Trace σ)) m') i t := by
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
  cases t with
  | zero => rfl
  | succ t =>
    show gateView (p i) ((A (iterate _ _ m)) i) (t + 1)
      = gateView (p i) ((A' (iterate _ _ m')) i) (t + 1)
    rw [gateView_succ, gateView_succ]
    split
    · exact hAB t (Nat.le_of_succ_le hth) _ _
        (fun i' t' ht' => ih t' (Nat.lt_succ_of_le ht') m m'
          (ht'.trans (Nat.le_of_succ_le hth))
          (Nat.lt_of_le_of_lt ht' (Nat.lt_of_succ_lt_succ hm))
          (Nat.lt_of_le_of_lt ht' (Nat.lt_of_succ_lt_succ hm')) i')
        i t (Nat.le_refl _)
    · exact ih t (Nat.lt_succ_self t) (m + 1) (m' + 1) (Nat.le_of_succ_le hth)
        (Nat.lt_of_succ_lt hm) (Nat.lt_of_succ_lt hm') i

/-! ## The head-dispatch causality walker (`causal_step`)

`causal_step` reads the agreement goal, takes the head constant of its
left wire term, and applies exactly the lemma the table names for that
head — an op head names its congruence lemma above, a knot head
(`fix_stream`/`fix_tick`, arriving Sched-spelled) names `causal_fix_*`
(recursion hypotheses introduced, to be closed by pre-established
per-knot causality `have`s). No alternative is ever *tried and
failed*, so no failing-branch `whnf` touches the (program-sized) wire
terms. -/

open Lean Elab Tactic Meta in
/-- Head-constant → congruence-lemma dispatch table. -/
private def causalDispatch : List (Name × Name) := [
  (``HydroSem.map, ``causal_map),
  (``HydroSem.filterMap, ``causal_filterMap),
  (``HydroSem.broadcast_closed, ``causal_broadcast_closed),
  (``HydroSem.demux, ``causal_demux),
  (``HydroSem.values, ``causal_values),
  (``HydroSem.into_keyed, ``causal_into_keyed),
  (``HydroSem.kassume_ordering, ``causal_kassume_ordering),
  (``HydroSem.kfold_early_stop, ``causal_kfold_early_stop),
  (``HydroSem.kget_max_key, ``causal_kget_max_key),
  (``HydroSem.tinto_keyed, ``causal_tinto_keyed),
  (``HydroSem.tassume_ordering, ``causal_tassume_ordering),
  (``HydroSem.reduce_watermark, ``causal_reduce_watermark),
  (``HydroSem.weaken_retries, ``causal_weaken_retries),
  (``HydroSem.union, ``causal_union),
  (``HydroSem.assume_ordering, ``causal_assume_ordering),
  (``HydroSem.fold, ``causal_fold),
  (``HydroSem.fold_monotone, ``causal_fold_monotone),
  (``HydroSem.snapshot, ``causal_snapshot),
  (``HydroSem.batch, ``causal_batch),
  (``HydroSem.mapTick, ``causal_mapTick),
  (``HydroSem.zipTick, ``causal_zipTick),
  (``HydroSem.defer_tick, ``causal_defer_tick),
  (``HydroSem.flattenOrdered, ``causal_flattenOrdered),
  (``HydroSem.flattenUnordered, ``causal_flattenUnordered),
  (``HydroSem.source_interval, ``causal_source_interval),
  (``HydroSem.firstTick, ``causal_firstTick),
  (``HydroSem.allTicks, ``causal_allTicks)]

open Lean Elab Tactic Meta in
elab "causal_step" "[" ids:Lean.Parser.Tactic.simpLemma,* "]" : tactic => do
  let g ← getMainGoal
  let ty ← instantiateMVars (← g.getType)
  -- the agreement relations all take the left wire as the
  -- second-to-last explicit argument
  let args := ty.getAppArgs
  if args.size < 2 then
    throwError "causal_step: not an agreement goal"
  let lhs := args[args.size - 2]!
  let head := lhs.getAppFn
  let .const headName _ := head |
    throwError "causal_step: left wire head is not a constant"
  if headName == ``HydroSem.fix_stream then
    evalTactic (← `(tactic|
      ((with_reducible refine causal_fix_stream _ ?_) <;> (first
        | with_reducible assumption
        | (intro _h' _hh' _x _y _hxy; simp only [$ids,*])))))
  else if headName == ``HydroSem.fix_tick then
    evalTactic (← `(tactic|
      ((with_reducible refine causal_fix_tick _ ?_) <;> (first
        | with_reducible assumption
        | (intro _h' _hh' _x _y _hxy; simp only [$ids,*])))))
  else
    match causalDispatch.lookup headName with
    | some lem =>
      evalTactic (← `(tactic|
        with_reducible apply $(mkIdent lem)))
    | none =>
      throwError "causal_step: no dispatch entry for {headName}"

end Hydro
