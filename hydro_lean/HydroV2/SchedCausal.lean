import HydroV2.Sched

/-!
# Machine-op causality (`SchedCausal`)

**Causality**: a machine op's output views at steps `≤ h` are a
function of its inputs' views at steps `≤ h`. This file proves it for
every `SchedSem` op — one congruence lemma per operator over the
view-agreement relations below (totality over the signature is the
cheat detector, per the `CorrSem` pattern) — and derives the two
knot-stabilization roots:

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

This is the missing generic ingredient named by `SCHED_AUDIT.md` F1:
it lets the telescope component of a knot's δ chain into a single
decision-environment witness (`SquareKnot.lean` consumes these roots;
`paxos_hsat` is the program-level payoff).
-/

namespace HydroV2

/-! ## View-agreement relations (one per machine carrier shape) -/

/-- Two step wires agree at all views up to `h`. -/
def VAgree {α : Type} (h : Nat) (x y : StepHist α) : Prop :=
  ∀ t, t ≤ h → x.view t = y.view t

/-- Stream carriers: member-wise view agreement up to `h`. -/
def SAgree {n : Nat} {α : Type} (h : Nat)
    (x y : Fin n → StepHist α) : Prop :=
  ∀ i, VAgree h (x i) (y i)

/-- Keyed carriers: per-(receiver, sender) view agreement up to `h`. -/
def KAgree {p c : Nat} {α : Type} (h : Nat)
    (x y : Fin p → Fin c → StepHist α) : Prop :=
  ∀ i j, VAgree h (x i j) (y i j)

/-- Fold carriers: source agreement up to `h`, identical read. -/
def FAgree {n : Nat} {α σ : Type} (h : Nat)
    (x y : Fin n → SchedFold α σ) : Prop :=
  ∀ i, VAgree h (x i).src (y i).src ∧ (x i).read = (y i).read

/-- Tick-trace carriers (`TickSingleton`/`TickStream`): entry
agreement at all steps up to `h`. -/
def TAgree {n : Nat} {σ : Type} (h : Nat)
    (x y : Fin n → Nat → Trace σ) : Prop :=
  ∀ i, ∀ t, t ≤ h → x i t = y i t

theorem VAgree.refl {α : Type} (h : Nat) (x : StepHist α) :
    VAgree h x x := fun _ _ => rfl

theorem SAgree.refl {n : Nat} {α : Type} (h : Nat)
    (x : Fin n → StepHist α) : SAgree h x x := fun _ _ _ => rfl

theorem KAgree.refl {p c : Nat} {α : Type} (h : Nat)
    (x : Fin p → Fin c → StepHist α) : KAgree h x x :=
  fun _ _ _ _ => rfl

theorem FAgree.refl {n : Nat} {α σ : Type} (h : Nat)
    (x : Fin n → SchedFold α σ) : FAgree h x x :=
  fun _ => ⟨fun _ _ => rfl, rfl⟩

theorem TAgree.refl {n : Nat} {σ : Type} (h : Nat)
    (x : Fin n → Nat → Trace σ) : TAgree h x x := fun _ _ _ => rfl

/-! Downward monotonicity: agreement at a horizon holds below it. -/

theorem VAgree.mono {α : Type} {h h' : Nat} {x y : StepHist α}
    (hh : h' ≤ h) (hv : VAgree h x y) : VAgree h' x y :=
  fun t ht => hv t (ht.trans hh)

theorem SAgree.mono {n : Nat} {α : Type} {h h' : Nat}
    {x y : Fin n → StepHist α} (hh : h' ≤ h) (hs : SAgree h x y) :
    SAgree h' x y := fun i => (hs i).mono hh

theorem KAgree.mono {p c : Nat} {α : Type} {h h' : Nat}
    {x y : Fin p → Fin c → StepHist α} (hh : h' ≤ h)
    (hk : KAgree h x y) : KAgree h' x y := fun i j => (hk i j).mono hh

theorem FAgree.mono {n : Nat} {α σ : Type} {h h' : Nat}
    {x y : Fin n → SchedFold α σ} (hh : h' ≤ h) (hf : FAgree h x y) :
    FAgree h' x y := fun i => ⟨(hf i).1.mono hh, (hf i).2⟩

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
  {pacing : (ℓ : L) → Fin (mem ℓ) → Nat → Bool}

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

theorem causal_broadcast {c p : L} {α : Type} [DecidableEq α]
    {ord : StrOrd} {ret : Retries} {h : Nat}
    (d : Fin (mem p) → Fin (mem c) → Nat → Nat)
    {s s' : (SchedSem L mem pacing).Stream c α ord ret}
    (hs : SAgree h s s') :
    KAgree h ((SchedSem L mem pacing).broadcast d s)
      ((SchedSem L mem pacing).broadcast d s') := by
  intro i j t ht
  match t with
  | 0 => rfl
  | t + 1 =>
    show ((s j).view t).take (cumMax (d i j) (t + 1))
      = ((s' j).view t).take (cumMax (d i j) (t + 1))
    rw [hs j t (Nat.le_of_succ_le ht)]

theorem causal_demux {c p : L} {α : Type} [DecidableEq α]
    {ord : StrOrd} {h : Nat}
    (d : Fin (mem p) → Fin (mem c) → Nat → Nat)
    {s s' : (SchedSem L mem pacing).Stream c (Nat × α) ord .exactlyOnce}
    (addr : Fin (mem p) → Nat) (hs : SAgree h s s') :
    KAgree h ((SchedSem L mem pacing).demux d s addr)
      ((SchedSem L mem pacing).demux d s' addr) := by
  intro i j t ht
  match t with
  | 0 => rfl
  | t + 1 =>
    show (((s j).view t).filterMap _).take (cumMax (d i j) (t + 1))
      = (((s' j).view t).filterMap _).take (cumMax (d i j) (t + 1))
    rw [hs j t (Nat.le_of_succ_le ht)]

theorem causal_values {p c : L} {α : Type} [DecidableEq α]
    {ord : StrOrd} {ret : Retries} {h : Nat}
    {k k' : (SchedSem L mem pacing).KeyedStream p c α ord ret}
    (hk : KAgree h k k') :
    SAgree h ((SchedSem L mem pacing).values k)
      ((SchedSem L mem pacing).values k') := by
  intro i t ht
  show mergeNView (k i) t = mergeNView (k' i) t
  exact mergeNView_congr (fun j => hk i j) t ht

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
    (ok : FoldOk ord ret g)
    {s s' : (SchedSem L mem pacing).Stream ℓ α ord ret}
    (hs : SAgree h s s') :
    FAgree h ((SchedSem L mem pacing).fold g init ok s)
      ((SchedSem L mem pacing).fold g init ok s') :=
  fun i => ⟨hs i, rfl⟩

theorem causal_fold_monotone {ℓ : L} {α σ : Type} [DecidableEq α]
    {ord : StrOrd} {ret : Retries} {h : Nat} (vo : ValueOrder σ)
    (g : σ → α → σ) (init : σ) (ok : FoldOk ord ret g)
    (hinfl : ∀ s x, vo.le s (g s x))
    {s s' : (SchedSem L mem pacing).Stream ℓ α ord ret}
    (hs : SAgree h s s') :
    FAgree h
      ((SchedSem L mem pacing).fold_monotone vo g init ok hinfl s)
      ((SchedSem L mem pacing).fold_monotone vo g init ok hinfl s') :=
  fun i => ⟨hs i, rfl⟩

end Ops

/-! ## Per-op causality (tick-boundary and in-tick ops) -/

section TickOps

variable {L : Type} {mem : L → Nat}
  {pacing : (ℓ : L) → Fin (mem ℓ) → Nat → Bool}

theorem causal_snapshot {ℓ : L} {α σ : Type} [DecidableEq α]
    {ord : StrOrd} {ret : Retries} {b : SingBound σ} {h : Nat}
    {s s' : (SchedSem L mem pacing).Singleton ℓ α σ ord ret b}
    (d : Unit) (hs : FAgree h s s') :
    TAgree h ((SchedSem L mem pacing).snapshot s d)
      ((SchedSem L mem pacing).snapshot s' d) := by
  intro i t ht
  show (tickSteps (pacing ℓ i) t).map
      (fun st => (s i).read ((s i).src.view st))
    = (tickSteps (pacing ℓ i) t).map
      (fun st => (s' i).read ((s' i).src.view st))
  refine List.map_congr_left (fun st hst => ?_)
  rw [(hs i).2, (hs i).1 st ((tickSteps_mem_le hst).trans ht)]

theorem causal_batch {ℓ : L} {α : Type} [DecidableEq α] {h : Nat}
    {s s' : (SchedSem L mem pacing).Stream ℓ α .noOrder .exactlyOnce}
    (d : Unit) (hs : SAgree h s s') :
    TAgree h ((SchedSem L mem pacing).batch s d)
      ((SchedSem L mem pacing).batch s' d) := by
  intro i t ht
  show batchesFrom ((s i).view) (tickSteps (pacing ℓ i) t) 0
    = batchesFrom ((s' i).view) (tickSteps (pacing ℓ i) t) 0
  exact batchesFrom_congr _
    (fun st hst => hs i st ((tickSteps_mem_le hst).trans ht)) 0

theorem causal_batch_ordered {ℓ : L} {α : Type} [DecidableEq α]
    {h : Nat}
    {s s' : (SchedSem L mem pacing).Stream ℓ α .totalOrder .exactlyOnce}
    (d : Unit) (hs : SAgree h s s') :
    TAgree h ((SchedSem L mem pacing).batch_ordered s d)
      ((SchedSem L mem pacing).batch_ordered s' d) := by
  intro i t ht
  show batchesFrom ((s i).view) (tickSteps (pacing ℓ i) t) 0
    = batchesFrom ((s' i).view) (tickSteps (pacing ℓ i) t) 0
  exact batchesFrom_congr _
    (fun st hst => hs i st ((tickSteps_mem_le hst).trans ht)) 0

theorem causal_assume_ordering_batch {ℓ : L} {α : Type} [DecidableEq α]
    {h : Nat}
    {bs bs' : (SchedSem L mem pacing).TickStream ℓ α .noOrder
      .exactlyOnce}
    (d : Unit) (hb : TAgree h bs bs') :
    TAgree h ((SchedSem L mem pacing).assume_ordering_batch bs d)
      ((SchedSem L mem pacing).assume_ordering_batch bs' d) := hb

theorem causal_mapBatchWith {ℓ : L} {α σ β : Type} [DecidableEq α]
    {h : Nat}
    {bs bs' : (SchedSem L mem pacing).TickStream ℓ α .totalOrder
      .exactlyOnce}
    {t t' : (SchedSem L mem pacing).TickSingleton ℓ σ .unbounded}
    (f : Fin (mem ℓ) → List α → σ → β)
    (hb : TAgree h bs bs') (ht : TAgree h t t') :
    TAgree h ((SchedSem L mem pacing).mapBatchWith bs t f)
      ((SchedSem L mem pacing).mapBatchWith bs' t' f) := by
  intro i u hu
  show (Trace.zip (bs i u) (t i u)).map (fun bx => f i bx.1 bx.2)
    = (Trace.zip (bs' i u) (t' i u)).map (fun bx => f i bx.1 bx.2)
  rw [hb i u hu, ht i u hu]

theorem causal_mapBatch {ℓ : L} {α β : Type} [DecidableEq α] {h : Nat}
    {bs bs' : (SchedSem L mem pacing).TickStream ℓ α .totalOrder
      .exactlyOnce}
    (f : Fin (mem ℓ) → List α → β) (hb : TAgree h bs bs') :
    TAgree h ((SchedSem L mem pacing).mapBatch bs f)
      ((SchedSem L mem pacing).mapBatch bs' f) := by
  intro i u hu
  show (bs i u).map (f i) = (bs' i u).map (f i)
  rw [hb i u hu]

theorem causal_mapBatchesWith {ℓ : L} {α σ β : Type} [DecidableEq α]
    [DecidableEq β] {h : Nat}
    {bs bs' : (SchedSem L mem pacing).TickStream ℓ α .noOrder
      .exactlyOnce}
    {t t' : (SchedSem L mem pacing).TickSingleton ℓ σ .unbounded}
    (f : Fin (mem ℓ) → α → σ → β)
    (hb : TAgree h bs bs') (ht : TAgree h t t') :
    TAgree h ((SchedSem L mem pacing).mapBatchesWith bs t f)
      ((SchedSem L mem pacing).mapBatchesWith bs' t' f) := by
  intro i u hu
  show (Trace.zip (bs i u) (t i u)).map _
    = (Trace.zip (bs' i u) (t' i u)).map _
  rw [hb i u hu, ht i u hu]

theorem causal_filterMapBatchesWith {ℓ : L} {α σ β : Type}
    [DecidableEq α] [DecidableEq β] {h : Nat}
    {bs bs' : (SchedSem L mem pacing).TickStream ℓ α .noOrder
      .exactlyOnce}
    {t t' : (SchedSem L mem pacing).TickSingleton ℓ σ .unbounded}
    (f : Fin (mem ℓ) → α → σ → Option β)
    (hb : TAgree h bs bs') (ht : TAgree h t t') :
    TAgree h ((SchedSem L mem pacing).filterMapBatchesWith bs t f)
      ((SchedSem L mem pacing).filterMapBatchesWith bs' t' f) := by
  intro i u hu
  show (Trace.zip (bs i u) (t i u)).map _
    = (Trace.zip (bs' i u) (t' i u)).map _
  rw [hb i u hu, ht i u hu]

theorem causal_scan_batches_across_ticks {ℓ : L} {α τ σ β : Type}
    [DecidableEq α] {h : Nat}
    {bs bs' : (SchedSem L mem pacing).TickStream ℓ α .totalOrder
      .exactlyOnce}
    {t t' : (SchedSem L mem pacing).TickSingleton ℓ τ .unbounded}
    (g : Fin (mem ℓ) → σ → List α → τ → σ × β) (init : σ)
    (hb : TAgree h bs bs') (ht : TAgree h t t') :
    TAgree h
      ((SchedSem L mem pacing).scan_batches_across_ticks bs t g init)
      ((SchedSem L mem pacing).scan_batches_across_ticks bs' t' g init)
    := by
  intro i u hu
  show scanAcrossTicksTrace _ init (Trace.zip (bs i u) (t i u))
    = scanAcrossTicksTrace _ init (Trace.zip (bs' i u) (t' i u))
  rw [hb i u hu, ht i u hu]

theorem causal_fold_batches_across_ticks_monotone {ℓ : L} {α σ : Type}
    [DecidableEq α] {h : Nat} (vo : ValueOrder σ)
    (g : Fin (mem ℓ) → σ → α → σ) (init : σ)
    (comm : ∀ i s x y, g i (g i s x) y = g i (g i s y) x)
    (hinfl : ∀ i s x, vo.le s (g i s x))
    {bs bs' : (SchedSem L mem pacing).TickStream ℓ α .noOrder
      .exactlyOnce}
    (hb : TAgree h bs bs') :
    TAgree h
      ((SchedSem L mem pacing).fold_batches_across_ticks_monotone vo g
        init comm hinfl bs)
      ((SchedSem L mem pacing).fold_batches_across_ticks_monotone vo g
        init comm hinfl bs') := by
  intro i u hu
  show foldAcrossTicksTrace _ init (bs i u)
    = foldAcrossTicksTrace _ init (bs' i u)
  rw [hb i u hu]

theorem causal_scan_batches_unordered_across_ticks {ℓ : L}
    {α τ σ β : Type} [DecidableEq α] {h : Nat}
    {bs bs' : (SchedSem L mem pacing).TickStream ℓ α .noOrder
      .exactlyOnce}
    {t t' : (SchedSem L mem pacing).TickSingleton ℓ τ .unbounded}
    (g : Fin (mem ℓ) → σ → Multiset α → τ → σ × β) (init : σ)
    (hb : TAgree h bs bs') (ht : TAgree h t t') :
    TAgree h
      ((SchedSem L mem pacing).scan_batches_unordered_across_ticks
        bs t g init)
      ((SchedSem L mem pacing).scan_batches_unordered_across_ticks
        bs' t' g init) := by
  intro i u hu
  show scanAcrossTicksTrace _ init (Trace.zip (bs i u) (t i u))
    = scanAcrossTicksTrace _ init (Trace.zip (bs' i u) (t' i u))
  rw [hb i u hu, ht i u hu]

theorem causal_scan_batches_unordered {ℓ : L} {α σ β : Type}
    [DecidableEq α] {h : Nat}
    {bs bs' : (SchedSem L mem pacing).TickStream ℓ α .noOrder
      .exactlyOnce}
    (g : Fin (mem ℓ) → σ → Multiset α → σ × β) (init : σ)
    (hb : TAgree h bs bs') :
    TAgree h ((SchedSem L mem pacing).scan_batches_unordered bs g init)
      ((SchedSem L mem pacing).scan_batches_unordered bs' g init) := by
  intro i u hu
  show scanAcrossTicksTrace _ init (bs i u)
    = scanAcrossTicksTrace _ init (bs' i u)
  rw [hb i u hu]

theorem causal_scan_batches_unordered₂ {ℓ : L} {α γ σ β : Type}
    [DecidableEq α] [DecidableEq γ] {h : Nat}
    {bs bs' : (SchedSem L mem pacing).TickStream ℓ α .noOrder
      .exactlyOnce}
    {cs cs' : (SchedSem L mem pacing).TickStream ℓ γ .noOrder
      .exactlyOnce}
    (g : Fin (mem ℓ) → σ → Multiset α → Multiset γ → σ × β) (init : σ)
    (hb : TAgree h bs bs') (hc : TAgree h cs cs') :
    TAgree h
      ((SchedSem L mem pacing).scan_batches_unordered₂ bs cs g init)
      ((SchedSem L mem pacing).scan_batches_unordered₂ bs' cs' g init)
    := by
  intro i u hu
  show scanAcrossTicksTrace _ init (Trace.zip (bs i u) (cs i u))
    = scanAcrossTicksTrace _ init (Trace.zip (bs' i u) (cs' i u))
  rw [hb i u hu, hc i u hu]

theorem causal_scan_across_ticks {ℓ : L} {α σ β : Type} {h : Nat}
    {t t' : (SchedSem L mem pacing).TickSingleton ℓ α .unbounded}
    (g : Fin (mem ℓ) → σ → α → σ × β) (init : σ)
    (ht : TAgree h t t') :
    TAgree h ((SchedSem L mem pacing).scan_across_ticks t g init)
      ((SchedSem L mem pacing).scan_across_ticks t' g init) := by
  intro i u hu
  show scanAcrossTicksTrace (g i) init (t i u)
    = scanAcrossTicksTrace (g i) init (t' i u)
  rw [ht i u hu]

theorem causal_mapTick {ℓ : L} {α β : Type} {h : Nat}
    {t t' : (SchedSem L mem pacing).TickSingleton ℓ α .unbounded}
    (f : Fin (mem ℓ) → α → β) (ht : TAgree h t t') :
    TAgree h ((SchedSem L mem pacing).mapTick t f)
      ((SchedSem L mem pacing).mapTick t' f) := by
  intro i u hu
  show (t i u).map (f i) = (t' i u).map (f i)
  rw [ht i u hu]

theorem causal_zipTick {ℓ : L} {α β : Type} {h : Nat}
    {a a' : (SchedSem L mem pacing).TickSingleton ℓ α .unbounded}
    {b b' : (SchedSem L mem pacing).TickSingleton ℓ β .unbounded}
    (ha : TAgree h a a') (hb : TAgree h b b') :
    TAgree h ((SchedSem L mem pacing).zipTick a b)
      ((SchedSem L mem pacing).zipTick a' b') := by
  intro i u hu
  show Trace.zip (a i u) (b i u) = Trace.zip (a' i u) (b' i u)
  rw [ha i u hu, hb i u hu]

theorem causal_fold_across_ticks_monotone {ℓ : L} {α σ : Type} {h : Nat}
    (vo : ValueOrder σ) (g : Fin (mem ℓ) → σ → α → σ) (init : σ)
    (hinfl : ∀ i s x, vo.le s (g i s x))
    {t t' : (SchedSem L mem pacing).TickSingleton ℓ α .unbounded}
    (ht : TAgree h t t') :
    TAgree h
      ((SchedSem L mem pacing).fold_across_ticks_monotone vo g init
        hinfl t)
      ((SchedSem L mem pacing).fold_across_ticks_monotone vo g init
        hinfl t') := by
  intro i u hu
  show foldAcrossTicksTrace (g i) init (t i u)
    = foldAcrossTicksTrace (g i) init (t' i u)
  rw [ht i u hu]

theorem causal_mapMonotone {ℓ : L} {σ τ : Type} {vo : ValueOrder σ}
    (vo' : ValueOrder τ) {h : Nat}
    {m m' : (SchedSem L mem pacing).TickSingleton ℓ σ (.monotonic vo)}
    (f : Fin (mem ℓ) → σ → τ)
    (hpres : ∀ i {a b}, vo.le a b → vo'.le (f i a) (f i b))
    (hm : TAgree h m m') :
    TAgree h ((SchedSem L mem pacing).mapMonotone vo' m f hpres)
      ((SchedSem L mem pacing).mapMonotone vo' m' f hpres) := by
  intro i u hu
  show (m i u).map (f i) = (m' i u).map (f i)
  rw [hm i u hu]

theorem causal_forgetBound {ℓ : L} {σ : Type} {vo : ValueOrder σ}
    {h : Nat}
    {m m' : (SchedSem L mem pacing).TickSingleton ℓ σ (.monotonic vo)}
    (hm : TAgree h m m') :
    TAgree h ((SchedSem L mem pacing).forgetBound m)
      ((SchedSem L mem pacing).forgetBound m') := hm

theorem causal_defer {ℓ : L} {σ : Type} {h : Nat} (init : σ)
    {t t' : (SchedSem L mem pacing).TickSingleton ℓ σ .unbounded}
    (ht : TAgree h t t') :
    TAgree h ((SchedSem L mem pacing).defer init t)
      ((SchedSem L mem pacing).defer init t') := by
  intro i u hu
  show init :: t i u = init :: t' i u
  rw [ht i u hu]

theorem causal_mapBatchesUnordered {ℓ : L} {α σ β : Type}
    [DecidableEq α] {h : Nat}
    {bs bs' : (SchedSem L mem pacing).TickStream ℓ α .noOrder
      .exactlyOnce}
    {t t' : (SchedSem L mem pacing).TickSingleton ℓ σ .unbounded}
    (f : Fin (mem ℓ) → Multiset α → σ → β)
    (hb : TAgree h bs bs') (ht : TAgree h t t') :
    TAgree h ((SchedSem L mem pacing).mapBatchesUnordered bs t f)
      ((SchedSem L mem pacing).mapBatchesUnordered bs' t' f) := by
  intro i u hu
  show (Trace.zip (bs i u) (t i u)).map _
    = (Trace.zip (bs' i u) (t' i u)).map _
  rw [hb i u hu, ht i u hu]

theorem causal_emitBatches {ℓ : L} {β : Type} [DecidableEq β] {h : Nat}
    {t t' : (SchedSem L mem pacing).TickSingleton ℓ (List β) .unbounded}
    (ht : TAgree h t t') :
    TAgree h ((SchedSem L mem pacing).emitBatches t)
      ((SchedSem L mem pacing).emitBatches t') := ht

theorem causal_emitMultisetBatches {ℓ : L} {β : Type} [DecidableEq β]
    {h : Nat}
    {t t' : (SchedSem L mem pacing).TickSingleton ℓ (Multiset β)
      .unbounded}
    (e : Fin (mem ℓ) → List (List β)) (ht : TAgree h t t') :
    TAgree h ((SchedSem L mem pacing).emitMultisetBatches t e)
      ((SchedSem L mem pacing).emitMultisetBatches t' e) := by
  intro i u hu
  show emitLin (t i u) (e i) = emitLin (t' i u) (e i)
  rw [ht i u hu]

theorem causal_emitBatchesUnordered {ℓ : L} {β : Type} [DecidableEq β]
    {h : Nat}
    {t t' : (SchedSem L mem pacing).TickSingleton ℓ (List β) .unbounded}
    (ht : TAgree h t t') :
    TAgree h ((SchedSem L mem pacing).emitBatchesUnordered t)
      ((SchedSem L mem pacing).emitBatchesUnordered t') := ht

/-- Timer verdicts are pure timing: the input wire is dataflow
documentation only. -/
theorem causal_timeout_snapshot {ℓ : L} {α : Type} [DecidableEq α]
    {ord : StrOrd} {ret : Retries} {h : Nat}
    {s s' : (SchedSem L mem pacing).Stream ℓ α ord ret}
    (d : TimerVerdicts (mem ℓ)) :
    TAgree h ((SchedSem L mem pacing).timeout_snapshot s d)
      ((SchedSem L mem pacing).timeout_snapshot s' d) :=
  fun _ _ _ => rfl

theorem causal_source_interval_batch {ℓ : L} {h : Nat}
    (d : TimingPulses (mem ℓ)) :
    TAgree h ((SchedSem L mem pacing).source_interval_batch (ℓ := ℓ) d)
      ((SchedSem L mem pacing).source_interval_batch (ℓ := ℓ) d) :=
  fun _ _ _ => rfl

end TickOps

/-! ## Per-op causality (`famHist` producers and knots) -/

section KnotOps

variable {L : Type} {mem : L → Nat}
  {pacing : (ℓ : L) → Fin (mem ℓ) → Nat → Bool}

theorem causal_sample_every {ℓ : L} {α : Type} [DecidableEq α] {h : Nat}
    {t t' : (SchedSem L mem pacing).TickSingleton ℓ (Option α)
      .unbounded}
    (d : SampleTimes (mem ℓ)) (ht : TAgree h t t') :
    SAgree h ((SchedSem L mem pacing).sample_every t d)
      ((SchedSem L mem pacing).sample_every t' d) := by
  intro i u hu
  show famFreeze (fun step j => sampleAtOpt (t j step) (d j)) u i
    = famFreeze (fun step j => sampleAtOpt (t' j step) (d j)) u i
  rw [famFreeze_congr_le u (fun u' hu' => funext fun j => by
    rw [ht j u' (hu'.trans hu)])]

theorem causal_allTicks {ℓ : L} {β : Type} [DecidableEq β]
    {ord : StrOrd} {h : Nat}
    {bs bs' : (SchedSem L mem pacing).TickStream ℓ β ord .exactlyOnce}
    (hb : TAgree h bs bs') :
    SAgree h ((SchedSem L mem pacing).allTicks bs)
      ((SchedSem L mem pacing).allTicks bs') := by
  intro i u hu
  show famFreeze (fun step j => (bs j step).flatten) u i
    = famFreeze (fun step j => (bs' j step).flatten) u i
  rw [famFreeze_congr_le u (fun u' hu' => funext fun j => by
    rw [hb j u' (hu'.trans hu)])]

/-- Shift-iterates of two causally-related step functions agree below
the horizon (both sides at the same iterate count). -/
theorem iterate_shift_congr {n : Nat} {α : Type} {h : Nat}
    {A B : (Fin n → StepHist α) → (Fin n → StepHist α)}
    (hAB : ∀ h', h' ≤ h → ∀ (x y : Fin n → StepHist α), SAgree h' x y →
      SAgree h' (A x) (B y)) :
    ∀ m, SAgree h
      (iterate (fun x j => ((A x) j).shift) (fun _ => StepHist.bot) m)
      (iterate (fun x j => ((B x) j).shift) (fun _ => StepHist.bot) m)
  | 0 => SAgree.refl h _
  | m + 1 => by
    intro i t ht
    match t with
    | 0 => rfl
    | t + 1 =>
      show (A (iterate _ _ m) i).view t = (B (iterate _ _ m) i).view t
      exact hAB h (Nat.le_refl _) _ _ (iterate_shift_congr hAB m) i t
        (Nat.le_of_succ_le ht)

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
      ((iterate (fun x j' => ((body x) j').shift)
        (fun _ => StepHist.bot) (t + 1)) j).view t) u i
    = famFreeze (fun t j =>
      ((iterate (fun x j' => ((body' x) j').shift)
        (fun _ => StepHist.bot) (t + 1)) j).view t) u i
  exact congrFun (famFreeze_congr_le u (fun u' hu' => funext fun j =>
    iterate_shift_congr
      (fun h'' hh'' x y hxy => hb h'' (hh''.trans (hu'.trans hu)) x y hxy)
      (u' + 1) j u' (Nat.le_refl _))) i

/-- Tick-iterates of two causally-related step functions agree below
the horizon. -/
theorem iterate_tickshift_congr {n : Nat} {σ : Type} {h : Nat}
    {A B : (Fin n → Nat → Trace σ) → (Fin n → Nat → Trace σ)}
    (hAB : ∀ h', h' ≤ h → ∀ (x y : Fin n → Nat → Trace σ),
      TAgree h' x y → TAgree h' (A x) (B y)) :
    ∀ m, TAgree h
      (iterate (fun x j u => match u with
        | 0 => ([] : Trace σ)
        | u + 1 => (A x) j u) (fun _ _ => []) m)
      (iterate (fun x j u => match u with
        | 0 => ([] : Trace σ)
        | u + 1 => (B x) j u) (fun _ _ => []) m)
  | 0 => TAgree.refl h _
  | m + 1 => by
    intro i t ht
    match t with
    | 0 => rfl
    | t + 1 =>
      show (A (iterate _ _ m)) i t = (B (iterate _ _ m)) i t
      exact hAB h (Nat.le_refl _) _ _ (iterate_tickshift_congr hAB m) i t
        (Nat.le_of_succ_le ht)

/-- Knot causality (tick). -/
theorem causal_fix_tick {ℓ : L} {σ : Type} {h : Nat} (d : Unit)
    {body body' : (SchedSem L mem pacing).TickSingleton ℓ σ .unbounded →
      (SchedSem L mem pacing).TickSingleton ℓ σ .unbounded}
    (hb : ∀ h', h' ≤ h →
      ∀ (x y : (SchedSem L mem pacing).TickSingleton ℓ σ .unbounded),
      TAgree h' x y → TAgree h' (body x) (body' y)) :
    TAgree h ((SchedSem L mem pacing).fix_tick d body)
      ((SchedSem L mem pacing).fix_tick d body') := by
  intro i u hu
  show (iterate (fun x j u' => match u' with
      | 0 => ([] : Trace σ)
      | u' + 1 => (body x) j u') (fun _ _ => []) (u + 1)) i u
    = (iterate (fun x j u' => match u' with
      | 0 => ([] : Trace σ)
      | u' + 1 => (body' x) j u') (fun _ _ => []) (u + 1)) i u
  exact iterate_tickshift_congr
    (fun h'' hh'' x y hxy => hb h'' (hh''.trans hu) x y hxy)
    (u + 1) i u (Nat.le_refl _)

end KnotOps

/-! ## Stage-stabilization roots

Shift-iterates of (hetero-)causal step functions are view-stable below
their iterate count: every stage of a knot shows the same views — at
the steps its own δ atoms read — as every later stage. These are the
roots the telescope chain lemmas stand on (`SquareKnot.lean` rewrites
square stage legs into these iterates via `stages_sr`). -/

/-- Stream root: views below both iterate counts agree. -/
theorem stab_iterate_shift {n : Nat} {α : Type} {h : Nat}
    {A A' : (Fin n → StepHist α) → (Fin n → StepHist α)}
    (hAB : ∀ h', h' ≤ h → ∀ (x y : Fin n → StepHist α),
      SAgree h' x y → SAgree h' (A x) (A' y)) :
    ∀ (t m m' : Nat), t ≤ h → t < m → t < m' → ∀ i,
      ((iterate (fun x j => ((A x) j).shift)
        (fun _ => StepHist.bot) m) i).view t
      = ((iterate (fun x j => ((A' x) j).shift)
        (fun _ => StepHist.bot) m') i).view t
  | t, m + 1, m' + 1, hth, hm, hm', i => by
    match t, hth, hm, hm' with
    | 0, _, _, _ => rfl
    | t + 1, hth, hm, hm' =>
      show (A (iterate _ _ m) i).view t = (A' (iterate _ _ m') i).view t
      exact hAB t (Nat.le_of_succ_le hth) _ _
        (fun i' t' ht' => stab_iterate_shift hAB t' m m'
          ((ht'.trans (Nat.le_of_lt_succ (Nat.lt_succ_of_le
            (Nat.le_refl t)))).trans (Nat.le_of_succ_le hth))
          (Nat.lt_of_le_of_lt ht' (Nat.lt_of_succ_lt_succ hm))
          (Nat.lt_of_le_of_lt ht' (Nat.lt_of_succ_lt_succ hm')) i')
        i t (Nat.le_refl _)
  termination_by t _ _ _ => t

/-- Tick root: entries below both iterate counts agree. -/
theorem stab_iterate_tick {n : Nat} {σ : Type} {h : Nat}
    {A A' : (Fin n → Nat → Trace σ) → (Fin n → Nat → Trace σ)}
    (hAB : ∀ h', h' ≤ h → ∀ (x y : Fin n → Nat → Trace σ),
      TAgree h' x y → TAgree h' (A x) (A' y)) :
    ∀ (t m m' : Nat), t ≤ h → t < m → t < m' → ∀ i,
      (iterate (τ := Fin n → Nat → Trace σ)
        (fun x j u => match u with
        | 0 => ([] : Trace σ)
        | u + 1 => (A x) j u)
        (fun _ _ => ([] : Trace σ)) m) i t
      = (iterate (τ := Fin n → Nat → Trace σ)
        (fun x j u => match u with
        | 0 => ([] : Trace σ)
        | u + 1 => (A' x) j u)
        (fun _ _ => ([] : Trace σ)) m') i t
  | t, m + 1, m' + 1, hth, hm, hm', i => by
    match t, hth, hm, hm' with
    | 0, _, _, _ => rfl
    | t + 1, hth, hm, hm' =>
      show (A (iterate (τ := Fin n → Nat → Trace σ)
          (fun x j u => match u with
          | 0 => ([] : Trace σ)
          | u + 1 => (A x) j u)
          (fun _ _ => ([] : Trace σ)) m)) i t
        = (A' (iterate (τ := Fin n → Nat → Trace σ)
          (fun x j u => match u with
          | 0 => ([] : Trace σ)
          | u + 1 => (A' x) j u)
          (fun _ _ => ([] : Trace σ)) m')) i t
      exact hAB t (Nat.le_of_succ_le hth) _ _
        (fun i' t' ht' => stab_iterate_tick hAB t' m m'
          (ht'.trans (Nat.le_of_succ_le hth))
          (Nat.lt_of_le_of_lt ht' (Nat.lt_of_succ_lt_succ hm))
          (Nat.lt_of_le_of_lt ht' (Nat.lt_of_succ_lt_succ hm')) i')
        i t (Nat.le_refl _)
  termination_by t _ _ _ => t

/-! ## The head-dispatch causality walker (`causal_step`)

`causal_step` reads the agreement goal, takes the head constant of its
left wire term, and applies exactly the lemma the table names for that
head — an op head names its congruence lemma above, a knot head
(`fix_stream`/`fix_tick`, arriving Sched-spelled) names `causal_fix_*`
(recursion hypotheses introduced, to be closed by pre-established
per-knot causality `have`s). No alternative is ever *tried and
failed*, so no failing-branch `whnf` touches the (program-sized) wire
terms. (Moved here from the retired `SquareHsat.lean`; the square-
carrier branches died with the square.) -/

open Lean Elab Tactic Meta in
/-- Head-constant → congruence-lemma dispatch table. -/
private def causalDispatch : List (Name × Name) := [
  (``HydroSem.map, ``causal_map),
  (``HydroSem.filterMap, ``causal_filterMap),
  (``HydroSem.broadcast, ``causal_broadcast),
  (``HydroSem.demux, ``causal_demux),
  (``HydroSem.values, ``causal_values),
  (``HydroSem.weaken_retries, ``causal_weaken_retries),
  (``HydroSem.union, ``causal_union),
  (``HydroSem.assume_ordering, ``causal_assume_ordering),
  (``HydroSem.fold, ``causal_fold),
  (``HydroSem.fold_monotone, ``causal_fold_monotone),
  (``HydroSem.snapshot, ``causal_snapshot),
  (``HydroSem.batch, ``causal_batch),
  (``HydroSem.batch_ordered, ``causal_batch_ordered),
  (``HydroSem.assume_ordering_batch, ``causal_assume_ordering_batch),
  (``HydroSem.mapBatchWith, ``causal_mapBatchWith),
  (``HydroSem.mapBatch, ``causal_mapBatch),
  (``HydroSem.mapBatchesWith, ``causal_mapBatchesWith),
  (``HydroSem.filterMapBatchesWith, ``causal_filterMapBatchesWith),
  (``HydroSem.scan_batches_across_ticks,
    ``causal_scan_batches_across_ticks),
  (``HydroSem.fold_batches_across_ticks_monotone,
    ``causal_fold_batches_across_ticks_monotone),
  (``HydroSem.scan_batches_unordered_across_ticks,
    ``causal_scan_batches_unordered_across_ticks),
  (``HydroSem.scan_batches_unordered, ``causal_scan_batches_unordered),
  (``HydroSem.scan_batches_unordered₂,
    ``causal_scan_batches_unordered₂),
  (``HydroSem.scan_across_ticks, ``causal_scan_across_ticks),
  (``HydroSem.mapTick, ``causal_mapTick),
  (``HydroSem.zipTick, ``causal_zipTick),
  (``HydroSem.fold_across_ticks_monotone,
    ``causal_fold_across_ticks_monotone),
  (``HydroSem.mapMonotone, ``causal_mapMonotone),
  (``HydroSem.forgetBound, ``causal_forgetBound),
  (``HydroSem.defer, ``causal_defer),
  (``HydroSem.mapBatchesUnordered, ``causal_mapBatchesUnordered),
  (``HydroSem.emitBatches, ``causal_emitBatches),
  (``HydroSem.emitMultisetBatches, ``causal_emitMultisetBatches),
  (``HydroSem.emitBatchesUnordered, ``causal_emitBatchesUnordered),
  (``HydroSem.timeout_snapshot, ``causal_timeout_snapshot),
  (``HydroSem.source_interval_batch, ``causal_source_interval_batch),
  (``HydroSem.sample_every, ``causal_sample_every),
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

end HydroV2
