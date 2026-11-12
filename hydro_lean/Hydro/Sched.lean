import Hydro.Sem
import Hydro.KeyedHash

/-!
# The step machine (`SchedSem`)

`SchedSem L mem pacing` is the concurrent operational semantics of a Hydro program. It is
**erased and concrete**: every grade executes as plain lists, with no `Multiset`,
`StutterSeq` or `RetryPool` anywhere in the machine. The quotients exist only in the
denotation (`Values`), and the transfer coupling (`Hydro/Transfer.lean`) relates this
machine's concrete runs to them through the grade types.

**Time** is one global step clock. A live stream wire is a `StepHist`: its concrete content
as of each step, prefix-monotone by construction. Fan-in (`values`, `union`, the network's
`mergeN`) takes no decision: at each step it appends the inputs' step increments in a fixed
order, so cross-sender interleaving is determined by delivery timing. Idle steps are legal
and observationally invisible (observations are indexed by tick and content, never by
step); they are how the model represents time passing without progress, and they give the
adversary unbounded lateness and asynchrony between members.

**The machine's nondeterminism** lives in its `…Dec` vocabulary and in `pacing`:

- `TransportDec` is a per-`(receiver, sender)` delivery cursor as a function of the step,
  read through `cumMax` so delivery is a monotone prefix of the sender's wire (per-pair
  FIFO; a stalled cursor is unbounded latency; `0` forever is silence). Transport never
  creates retries: `AtLeastOnce` duplicates are content born at sampling sites.
- `IntervalDec` and `ClockDec` are per-tick scripts (`TickScript`): the interval timer's fire
  counts and the clock's readings at each of the member's ticks.
- `pacing : Pacing L mem` says what each member of each location does at each step —
  `idle` (inert), `stutter` (its asynchronous dataflow progresses, its tick bodies do not
  run) or `tick` (its tick bodies run) — independently across cluster members. A DFIR tick
  is "stutters, then a tick"; a sleeping process is `idle`.

Content sites (`batch`, `snapshot`, `assume_ordering`, `fix`, …) take `Unit`: the machine
consumes its buffers in arrival order, a tick consumes everything available
(`batchesFrom`), a snapshot reads the fold's live value at the tick's step, and
`assume_ordering` is an identity operator because the machine has one real order.

**Floors.** Network delivery (`StepHist.deliver`) and `defer_tick` each read their source
one step back: at a ticking step `t+1` the receiver sees the sender's wire as of `t`
(`gateView`), and a tick of `defer_tick` releases from the buffer as of the previous step.
The floors exclude zero-time round trips, under which the Kleene diagonal has no stable
value. A knot carries no floor: its wire at step `t` reads stage `t + 1` of the body's Kleene
iteration at `t`, and every
cycle through it passes a network hop or a `defer_tick` (Rust's `complete` rule, DFIR's
stratification within one tick).

**Tick-domain carriers are raw per-step views** (`Fin (mem ℓ) → Nat → Trace σ`, no
monotonicity bundle): every tick op is pointwise per step, and `famFreeze` re-establishes
prefix-monotonicity at the tick→stream boundaries (`allTicks`, `kallTicks`, `fix_stream`),
family-atomically, so a frozen family view is always a single raw family view.
-/

namespace Hydro

/-! ## Step histories -/

/-- A live stream wire at the machine: its concrete content as of each global step,
prefix-monotone because buffers only grow. -/
structure StepHist (α : Type) where
  view : Nat → List α
  mono : ∀ t, view t <+: view (t + 1)

namespace StepHist

/-- `mono` across any number of steps. -/
theorem mono_le {α : Type} (h : StepHist α) {t t' : Nat} (ht : t ≤ t') :
    h.view t <+: h.view t' := by
  induction t' with
  | zero => cases Nat.le_zero.mp ht; exact List.prefix_refl _
  | succ t' ih =>
    rcases Nat.lt_or_ge t (t' + 1) with hlt | hge
    · exact (ih (Nat.lt_succ_iff.mp hlt)).trans (h.mono t')
    · cases Nat.le_antisymm ht hge; exact List.prefix_refl _

/-- A step history cut at horizon `T`: the view is frozen from `T` on. The machine-only
headlines truncate their inputs here and carry the outputs back by causality
(`Dag.denote_causal`). -/
def truncate {α : Type} (h : StepHist α) (T : Nat) : StepHist α :=
  ⟨fun t => h.view (min t T), fun t => h.mono_le (min_le_min_right T (Nat.le_succ t))⟩

/-- The empty wire. -/
def bot {α : Type} : StepHist α := ⟨fun _ => [], fun _ => List.prefix_refl _⟩

/-- A wire whose whole content is present from step 0 (a local source). -/
def const {α : Type} (l : List α) : StepHist α :=
  ⟨fun _ => l, fun _ => List.prefix_refl _⟩

/-- `map` on every step's view; prefix-monotonicity is preserved pointwise. -/
def map {α β : Type} (f : α → β) (h : StepHist α) : StepHist β :=
  ⟨fun t => (h.view t).map f, fun t => List.IsPrefix.map f (h.mono t)⟩

/-- `filterMap` on every step's view. -/
def filterMap {α β : Type} (f : α → Option β) (h : StepHist α) :
    StepHist β :=
  ⟨fun t => (h.view t).filterMap f, fun t => prefix_filterMap f (h.mono t)⟩

/-- The step increment: what arrived between steps `t` and `t+1`. -/
def inc {α : Type} (h : StepHist α) (t : Nat) : List α :=
  (h.view (t + 1)).drop (h.view t).length

/-- The next view is the current view followed by the increment. -/
theorem view_succ {α : Type} (h : StepHist α) (t : Nat) :
    h.view (t + 1) = h.view t ++ h.inc t := by
  obtain ⟨e, he⟩ := h.mono t
  rw [inc, ← he, List.drop_left]

end StepHist

/-! ## Delivery (ordered transport) -/

/-- The cumulative max of a cursor. Delivery cursors are read through it so that a
delivered message stays delivered whatever the raw cursor does. -/
def cumMax (c : Nat → Nat) : Nat → Nat
  | 0 => c 0
  | t + 1 => max (cumMax c t) (c (t + 1))

/-- `cumMax` is monotone in the step. -/
theorem cumMax_mono (c : Nat → Nat) (t : Nat) :
    cumMax c t ≤ cumMax c (t + 1) := Nat.le_max_left _ _

/-- `take n` of a prefix is a prefix of `take n`. -/
theorem prefix_take_prefix {α : Type _} {l l' : List α} (n : Nat)
    (h : l <+: l') : l.take n <+: l'.take n := by
  rw [List.prefix_iff_eq_take.mp h, List.take_take]
  exact take_prefix_take (Nat.min_le_left _ _)

/-! ### Pacing

What a member does at a step. -/

/-- `idle`: inert — a sleeping process; nothing it holds changes. `stutter`: active without
a tick — network deliveries land and knot feedback propagates, but no tick body runs.
`tick`: active, and its tick bodies run. -/
inductive Pace
  | idle
  | stutter
  | tick
  deriving DecidableEq, Repr

namespace Pace

/-- Whether anything moves at this member this step. -/
def active : Pace → Bool
  | .idle => false
  | .stutter => true
  | .tick => true

/-- Whether the tick bodies run. -/
def ticks : Pace → Bool
  | .tick => true
  | _ => false

/-- The always-active pace with the given tick flag. -/
def ofBool : Bool → Pace
  | true => .tick
  | false => .stutter

@[simp] theorem ticks_ofBool (b : Bool) : (ofBool b).ticks = b := by cases b <;> rfl
@[simp] theorem active_ofBool (b : Bool) : (ofBool b).active = true := by cases b <;> rfl
@[simp] theorem active_idle : active .idle = false := rfl
@[simp] theorem active_stutter : active .stutter = true := rfl
@[simp] theorem active_tick : active .tick = true := rfl
@[simp] theorem ticks_idle : ticks .idle = false := rfl
@[simp] theorem ticks_stutter : ticks .stutter = false := rfl
@[simp] theorem ticks_tick : ticks .tick = true := rfl

end Pace

/-- The pacing of a run: what each member of each location does at each step. -/
abbrev Pacing (L : Type) (mem : L → Nat) := (ℓ : L) → Fin (mem ℓ) → Nat → Pace

namespace Pacing

variable {L : Type} {mem : L → Nat}

/-- Nobody ever active. The compiled graph's data terms are elaborated at
`SchedSem L mem Pacing.idle`: the instance refers to a pacing only through its carrier types,
which do not depend on it, so the graph is pacing-free and its denotation
(`Dag.denote pacing …`) is taken at any pacing (`M.dag_denote`). -/
def idle : Pacing L mem := fun _ _ _ => .idle

/-- The member's tick skeleton: the steps at which its tick bodies run. -/
def ticks (pacing : Pacing L mem) (ℓ : L) (i : Fin (mem ℓ)) : Nat → Bool := fun t => (pacing ℓ i t).ticks

/-- The member's activity skeleton: the steps at which anything moves. -/
def active (pacing : Pacing L mem) (ℓ : L) (i : Fin (mem ℓ)) : Nat → Bool := fun t => (pacing ℓ i t).active

/-- The always-active pacing with the given tick skeleton. -/
def ofTicks (p : (ℓ : L) → Fin (mem ℓ) → Nat → Bool) : Pacing L mem := fun ℓ i t => .ofBool (p ℓ i t)

@[simp] theorem ticks_ofTicks (p : (ℓ : L) → Fin (mem ℓ) → Nat → Bool) (ℓ : L) (i : Fin (mem ℓ)) :
    (ofTicks p).ticks ℓ i = p ℓ i := funext fun t => Pace.ticks_ofBool _

@[simp] theorem active_ofTicks (p : (ℓ : L) → Fin (mem ℓ) → Nat → Bool) (ℓ : L) (i : Fin (mem ℓ)) :
    (ofTicks p).active ℓ i = fun _ => true := funext fun t => Pace.active_ofBool _

end Pacing

/-! ### Gating

A member that is not active at a step is **inert**: nothing it holds changes and nothing it
emits grows. Every per-step operator is pointwise in its inputs' views and inherits this.
The primitive that moves content across steps on its own, network delivery, is gated by the
receiving member's activity skeleton through `gateView`: at an active step `t+1` the gated
view presents the source as of `t` (the `+1` floor), at an inactive step it is frozen. The
ungated primitive (`gateView_true`) is the special case of a member active at every step.
Because of this gate, a sleeping process in a wired run coincides with an inactive member of
the lock-step machine (`Project/Model/WiredRun.lean`). -/

/-- The gated view of a per-step source: step 0 sees nothing; a step `t+1` at which `p`
holds sees the source as of `t`; a step at which `p` fails repeats the previous view. -/
def gateView {β : Type} (p : Nat → Bool) (f : Nat → List β) : Nat → List β
  | 0 => []
  | t + 1 => if p (t + 1) then f t else gateView p f t

/-- `gateView` at a successor step, unfolded. -/
theorem gateView_succ {β : Type} (p : Nat → Bool) (f : Nat → List β) (t : Nat) :
    gateView p f (t + 1) = if p (t + 1) then f t else gateView p f t := rfl

/-- A gated view of a prefix-monotone source is a prefix of the source's current view. -/
theorem gateView_prefix {β : Type} (p : Nat → Bool) (f : Nat → List β)
    (hf : ∀ t, f t <+: f (t + 1)) : ∀ t, gateView p f t <+: f t
  | 0 => List.nil_prefix
  | t + 1 => by
    rw [gateView_succ]
    split
    · exact hf t
    · exact (gateView_prefix p f hf t).trans (hf t)

/-- A gated view of a prefix-monotone source is prefix-monotone. -/
theorem gateView_mono {β : Type} (p : Nat → Bool) (f : Nat → List β)
    (hf : ∀ t, f t <+: f (t + 1)) (t : Nat) : gateView p f t <+: gateView p f (t + 1) := by
  rw [gateView_succ (t := t)]
  split
  · exact gateView_prefix p f hf t
  · exact List.prefix_refl _

/-- Gated views agree below a horizon when the sources agree strictly below it. -/
theorem gateView_congr {β : Type} (p : Nat → Bool) (f g : Nat → List β) (h : Nat)
    (hfg : ∀ t, t < h → f t = g t) : ∀ t, t ≤ h → gateView p f t = gateView p g t
  | 0, _ => rfl
  | t + 1, ht => by
    rw [gateView_succ, gateView_succ, hfg t ht, gateView_congr p f g h hfg t (Nat.le_of_succ_le ht)]

/-- Once the source is stable from `U` on and the member has been active at some
`U' + 1 ≥ U + 1`, the gated view holds the stable value from there on. -/
theorem gateView_stable {β : Type} (p : Nat → Bool) (f : Nat → List β) {U U' : Nat}
    (hf : ∀ u, U ≤ u → f u = f U) (hU : U ≤ U') (hp : p (U' + 1) = true) :
    ∀ t, U' + 1 ≤ t → gateView p f t = f U := by
  intro t ht
  obtain ⟨k, rfl⟩ := Nat.exists_eq_add_of_le ht
  induction k with
  | zero => rw [gateView_succ, hp]; exact hf U' hU; done
  | succ k ih =>
    rw [show U' + 1 + (k + 1) = (U' + 1 + k) + 1 from by omega, gateView_succ]
    split
    · exact hf _ (by omega)
    · exact ih (by omega)

/-- `gateView_mono` across any number of steps. -/
theorem gateView_mono_le {β : Type} (p : Nat → Bool) (f : Nat → List β)
    (hf : ∀ t, f t <+: f (t + 1)) {k k' : Nat} (hk : k ≤ k') : gateView p f k <+: gateView p f k' := by
  induction k' with
  | zero => cases Nat.le_zero.mp hk; exact List.prefix_refl _
  | succ k' ih =>
    rcases Nat.lt_or_ge k (k' + 1) with hlt | hge
    · exact (ih (Nat.lt_succ_iff.mp hlt)).trans (gateView_mono p f hf k')
    · cases Nat.le_antisymm hk hge; exact List.prefix_refl _

/-- Ungated: a member active at every step sees the source one step late. -/
theorem gateView_true {β : Type} (f : Nat → List β) (t : Nat) :
    gateView (fun _ => true) f (t + 1) = f t := by
  rw [gateView_succ]; rfl

/-- Network delivery: the prefix of the sender's wire that has reached the receiver by each
step, under a step cursor `c` and the receiver's activity skeleton `p`. **Delivery takes at
least one step**: at an active step `t+1` the receiver sees a `cumMax c` prefix of the
sender's wire as of `t`, and step 0 sees nothing. **An inactive receiver receives nothing**;
it drains its inbox when it runs. The floor is structural and the cursor keeps all its
freedom above it, so every cycle through the network advances the step clock and knot
stabilization is program-independent. -/
def StepHist.deliver {α : Type} (h : StepHist α) (p : Nat → Bool) (c : Nat → Nat) :
    StepHist α where
  view := gateView p (fun t => (h.view t).take (cumMax c (t + 1)))
  mono := gateView_mono p _ (fun t =>
    (prefix_take_prefix _ (h.mono t)).trans (take_prefix_take (cumMax_mono c (t + 1))))

/-! ## Fan-in

Fan-in takes no decision: at each step the inputs' increments are appended in a fixed
order, so the interleaving is determined by delivery timing. -/

/-- The view of two merged wires: both initial views at step 0, then both increments per
step, the first input's first. -/
def merge2View {α : Type} (a b : StepHist α) : Nat → List α
  | 0 => a.view 0 ++ b.view 0
  | t + 1 => merge2View a b t ++ a.inc t ++ b.inc t

/-- Two wires merged (`union`). -/
def merge2 {α : Type} (a b : StepHist α) : StepHist α :=
  ⟨merge2View a b,
   fun t => ⟨a.inc t ++ b.inc t, (List.append_assoc _ _ _).symm⟩⟩

/-- The view of a merged family of wires (the network's keyed fan-in): at each step every
sender's increment, in sender order within the step. -/
def mergeNView {m : Nat} {α : Type} (k : Fin m → StepHist α) :
    Nat → List α
  | 0 => (List.finRange m).flatMap (fun j => (k j).view 0)
  | t + 1 => mergeNView k t
      ++ (List.finRange m).flatMap (fun j => (k j).inc t)

/-- A family of wires merged. -/
def mergeN {m : Nat} {α : Type} (k : Fin m → StepHist α) : StepHist α :=
  ⟨mergeNView k, fun _t => ⟨_, rfl⟩⟩

/-- The ticks appended to a step-indexed trace at step `t`: the whole trace at `0`, and for
a step-monotone trace the new suffix thereafter. -/
def incTrace {σ : Type} (tr : Nat → Trace σ) : Nat → List σ
  | 0 => tr 0
  | t + 1 => (tr (t + 1)).drop (tr t).length

theorem incTrace_zero {σ : Type} (tr : Nat → Trace σ) : incTrace tr 0 = tr 0 := rfl
theorem incTrace_succ {σ : Type} (tr : Nat → Trace σ) (t : Nat) :
    incTrace tr (t + 1) = (tr (t + 1)).drop (tr t).length := rfl

/-- `defer_tick`'s released count as of step `k`: a tick releases the buffered entry if there
is one. `len k` is the input's length read at step `k`; the buffer is `init :: input` beyond
the released count, so it is nonempty iff `released ≤ len`. For an input at most one tick
behind the tick count — every Rust-expressible one — this is the tick count
(`deferReleased_eq_ticks` in `Hydro/SchedPaced.lean`); the step form is what the production
FIFO computes on any input, and so what the correspondence proof needs. -/
def deferReleased (p : Nat → Bool) (len : Nat → Nat) : Nat → Nat
  | 0 => if p 0 then 1 else 0
  | k + 1 =>
    if p (k + 1) && decide (deferReleased p len k ≤ len (k + 1)) then deferReleased p len k + 1
    else deferReleased p len k

/-- `deferReleased` at a successor step, unfolded. -/
theorem deferReleased_succ (p : Nat → Bool) (len : Nat → Nat) (k : Nat) :
    deferReleased p len (k + 1) =
      if p (k + 1) && decide (deferReleased p len k ≤ len (k + 1)) then deferReleased p len k + 1
      else deferReleased p len k := rfl

/-- The released count never decreases. -/
theorem deferReleased_le_succ (p : Nat → Bool) (len : Nat → Nat) (k : Nat) :
    deferReleased p len k ≤ deferReleased p len (k + 1) := by
  rw [deferReleased_succ]; split <;> omega

/-- The released count grows by at most one per step. -/
theorem deferReleased_succ_le (p : Nat → Bool) (len : Nat → Nat) (k : Nat) :
    deferReleased p len (k + 1) ≤ deferReleased p len k + 1 := by
  rw [deferReleased_succ]; split <;> omega

/-- At most one entry is released at step 0. -/
theorem deferReleased_zero_le (p : Nat → Bool) (len : Nat → Nat) : deferReleased p len 0 ≤ 1 := by
  show (if p 0 then 1 else 0) ≤ 1; split <;> omega

/-- A non-tick step releases nothing. -/
theorem deferReleased_of_not (p : Nat → Bool) (len : Nat → Nat) (k : Nat) (h : p (k + 1) = false) :
    deferReleased p len (k + 1) = deferReleased p len k := by
  rw [deferReleased_succ, h]; rfl

/-- A non-tick step 0 releases nothing. -/
theorem deferReleased_zero_of_not (p : Nat → Bool) (len : Nat → Nat) (h : p 0 = false) :
    deferReleased p len 0 = 0 := by
  show (if p 0 then 1 else 0) = 0; rw [h]; rfl

/-- `deferReleased_le_succ` across any number of steps. -/
theorem deferReleased_mono (p : Nat → Bool) (len : Nat → Nat) {k k' : Nat} (h : k ≤ k') :
    deferReleased p len k ≤ deferReleased p len k' := by
  induction k' with
  | zero => cases Nat.le_zero.mp h; exact Nat.le_refl _
  | succ k' ih =>
    rcases Nat.lt_or_eq_of_le h with hlt | heq
    · exact (ih (Nat.lt_succ_iff.mp hlt)).trans (deferReleased_le_succ p len k')
    · rw [heq]

/-- The released count never exceeds the buffer `init :: input`: a tick releases only what is
there. Needs the input's length to be monotone in the step. -/
theorem deferReleased_le_len_succ (p : Nat → Bool) (len : Nat → Nat) (hlen : ∀ k, len k ≤ len (k + 1)) :
    ∀ k, deferReleased p len k ≤ len k + 1
  | 0 => Nat.le_trans (deferReleased_zero_le p len) (Nat.le_add_left 1 _)
  | k + 1 => by
    rw [deferReleased_succ]
    have := deferReleased_le_len_succ p len hlen k
    have := hlen k
    split
    · rename_i h; simp only [Bool.and_eq_true, decide_eq_true_eq] at h; omega
    · omega

/-- The released count only reads the input's lengths at steps `≤ k`. -/
theorem deferReleased_congr (p : Nat → Bool) {len len' : Nat → Nat} :
    ∀ k, (∀ u, u ≤ k → len u = len' u) → deferReleased p len k = deferReleased p len' k
  | 0, _ => rfl
  | k + 1, h => by
    rw [deferReleased_succ, deferReleased_succ, deferReleased_congr p k (fun u hu => h u (Nat.le_succ_of_le hu)),
      h (k + 1) (Nat.le_refl _)]

/-! ## The tick skeleton -/

/-- The steps at which a member has ticked, as of step `t`. -/
def tickSteps (p : Nat → Bool) (t : Nat) : List Nat :=
  (List.range (t + 1)).filter p

/-- The tick steps grow by prefix. -/
theorem tickSteps_prefix (p : Nat → Bool) (t : Nat) :
    tickSteps p t <+: tickSteps p (t + 1) := by
  unfold tickSteps
  rw [List.range_succ (n := t + 1), List.filter_append]
  exact ⟨_, rfl⟩

/-- `tickSteps_prefix` across any number of steps. -/
theorem tickSteps_prefix_le (p : Nat → Bool) {t t' : Nat} (h : t ≤ t') :
    tickSteps p t <+: tickSteps p t' := by
  induction t' with
  | zero => cases Nat.le_zero.mp h; exact List.prefix_refl _
  | succ t' ih =>
    rcases Nat.lt_or_ge t (t' + 1) with hlt | hge
    · exact (ih (Nat.lt_succ_iff.mp hlt)).trans (tickSteps_prefix p t')
    · cases Nat.le_antisymm h hge; exact List.prefix_refl _

/-- A member that has not ticked by `t'` has not ticked by any earlier `t`. -/
theorem tickSteps_isEmpty_of_le (p : Nat → Bool) {t t' : Nat} (h : t ≤ t')
    (he : (tickSteps p t').isEmpty = true) : (tickSteps p t).isEmpty = true := by
  rw [List.isEmpty_iff] at he ⊢
  exact List.prefix_nil.mp (he ▸ tickSteps_prefix_le p h)

/-! ## The interval timer at the machine

`source_interval(_delayed)` is a `TotalOrder × AtLeastOnce` stream of `()`. At the machine
its arrivals are timing: a per-tick script of fire counts, how many fire instants fell since
the member's previous tick (`tokio::time::interval`'s default `MissedTickBehavior::Burst`
delivers every missed instant at once; `location/mod.rs:1500–1545`). The fires of the
member's `k`-th tick arrive at that tick's step: only `batch` reads the stream, at a tick, so
arriving between ticks and arriving at the tick are indistinguishable. -/

/-- The total of the first `k` entries of a per-tick count script. -/
def scriptSum (sc : Nat → Nat) (k : Nat) : Nat := (scriptTrace sc k).sum

/-- `scriptSum` is monotone in `k`. -/
theorem scriptSum_le {sc : Nat → Nat} {k k' : Nat} (h : k ≤ k') : scriptSum sc k ≤ scriptSum sc k' := by
  obtain ⟨e, he⟩ := scriptTrace_prefix sc h
  show (scriptTrace sc k).sum ≤ (scriptTrace sc k').sum
  rw [← he, List.sum_append]
  exact Nat.le_add_right _ _

/-- `scriptSum` at a successor count adds the next entry. -/
theorem scriptSum_succ (sc : Nat → Nat) (k : Nat) : scriptSum sc (k + 1) = scriptSum sc k + sc k := by
  show (scriptTrace sc (k + 1)).sum = (scriptTrace sc k).sum + sc k
  rw [scriptTrace_succ, List.sum_append, List.sum_singleton]

/-- A shorter replicate is a prefix of a longer one. -/
theorem replicate_prefix_of_le {α : Type} (x : α) {a b : Nat} (h : a ≤ b) :
    List.replicate a x <+: List.replicate b x :=
  ⟨List.replicate (b - a) x, by rw [List.replicate_append_replicate, Nat.add_sub_cancel' h]⟩

/-- The interval stream's history: the fires of every tick so far, as `()`s. -/
def intervalHist (p : Nat → Bool) (sc : Nat → Nat) : StepHist Unit :=
  ⟨fun t => List.replicate (scriptSum sc (tickSteps p t).length) (),
   fun t => replicate_prefix_of_le () (scriptSum_le (tickSteps_prefix p t).length_le)⟩

/-- Per-tick batches: at each tick step, everything available and not yet consumed. -/
def batchesFrom {α : Type} (src : Nat → List α) :
    List Nat → Nat → List (List α)
  | [], _ => []
  | s :: rest, consumed =>
    (src s).drop consumed :: batchesFrom src rest (src s).length

/-! ## Re-monotonization at tick→stream boundaries -/

/-- Freeze a raw per-member view sequence into prefix-monotone histories,
**family-atomically**: advance only while every member's next view extends its current one.
Every frozen family view is therefore one of the raw family views (`famFreeze_eq_raw`), and
the transfer coupling can identify the single step it came from. On flows whose tick traces
grow by prefix, member-uniformly, `famFreeze` returns its input unchanged; it is total on
arbitrary ones. -/
def famFreeze {n : Nat} {β : Type} [DecidableEq β]
    (h : Nat → Fin n → List β) : Nat → Fin n → List β
  | 0 => h 0
  | t + 1 =>
    let prev := famFreeze h t
    if (List.finRange n).all
        (fun i => (prev i).isPrefixOf (h (t + 1) i))
    then h (t + 1) else prev

/-- The frozen views are prefix-monotone, per member. -/
theorem famFreeze_mono {n : Nat} {β : Type} [DecidableEq β]
    (h : Nat → Fin n → List β) (i : Fin n) (t : Nat) :
    famFreeze h t i <+: famFreeze h (t + 1) i := by
  show famFreeze h t i <+:
    (if (List.finRange n).all
        (fun j => (famFreeze h t j).isPrefixOf (h (t + 1) j))
     then h (t + 1) else famFreeze h t) i
  split
  · next hp =>
    exact List.isPrefixOf_iff_prefix.mp
      (List.all_eq_true.mp hp i (List.mem_finRange i))
  · exact List.prefix_refl _

/-- Every frozen family view is one of the raw family views, at a step `k ≤ t`: no splicing
and no member skew. -/
theorem famFreeze_eq_raw {n : Nat} {β : Type} [DecidableEq β]
    (h : Nat → Fin n → List β) :
    ∀ t, ∃ k, k ≤ t ∧ famFreeze h t = h k
  | 0 => ⟨0, Nat.le_refl _, rfl⟩
  | t + 1 => by
    show ∃ k, k ≤ t + 1 ∧
      (if (List.finRange n).all
          (fun i => (famFreeze h t i).isPrefixOf (h (t + 1) i))
       then h (t + 1) else famFreeze h t) = h k
    split
    · exact ⟨t + 1, Nat.le_refl _, rfl⟩
    · obtain ⟨k, hk, he⟩ := famFreeze_eq_raw h t
      exact ⟨k, Nat.le_succ_of_le hk, he⟩

/-- `f t`, by cases on `t`: a **reduction gate**. The kernel cannot unfold `stepGate f t` for
a symbolic `t` (the `Nat.casesOn` is stuck), so a tick knot applied at an unknown step is
inert under definitional unfolding. Without it the diagonal `iterate body ⊥ (t + 1) i t`
unfolds once for every symbolic application (`t + 1` is `Nat.succ t`) and the kernel
evaluates the knot's whole body, which makes the production graphs' denotation lemmas
(`HydroGenProject`) infeasible. The stream knot needs no gate because `famFreeze` recurses on
the step. Semantically an identity operator (`stepGate_eq`). -/
def stepGate {α : Sort u} (f : Nat → α) : Nat → α
  | 0 => f 0
  | t + 1 => f (t + 1)

@[simp] theorem stepGate_eq {α : Sort u} (f : Nat → α) (t : Nat) : stepGate f t = f t := by
  cases t <;> rfl

/-- The frozen family as per-member step histories. -/
def famHist {n : Nat} {β : Type} [DecidableEq β]
    (h : Nat → Fin n → List β) : Fin n → StepHist β :=
  fun i => ⟨fun t => famFreeze h t i, famFreeze_mono h i⟩

/-! ## The fold singleton: the live value, per step

A folded live singleton at the machine is its current value as of each step: the fold, in
arrival order, of whatever the source wire has delivered by that step. No commutativity is
needed to run; the grade obligations are consumed by the transfer proof. A tick read
(`snapshot`) takes the value at the tick's step. This mirrors, step by step, the
runtime's representation of a root singleton or optional — a feed of accumulator updates
whose consumer holds the latest (`root_repr_is_tombstoned_update_feed` in
`compile/ir/mod.rs`) — with the feed's successive updates being this function's successive
values. Asynchrony is entirely upstream, in the delivery cursors that decide how much has
been folded by each step; nothing is stored but the value. -/

/-- The live value of a fold over a source wire, per step. -/
def schedFold {α σ : Type} (g : σ → α → σ) (init : σ) (src : StepHist α) :
    Nat → σ :=
  fun t => (src.view t).foldl g init

/-- `across_ticks(|s| s.fold(…))` at the machine: the fold's state retained across ticks, one
value per realized tick. -/
def tfoldStep {α σ : Type} (g : σ → α → σ) (s : σ) (b : Array α) : σ × σ :=
  (b.foldl g s, b.foldl g s)

/-- The scan of `tfoldStep` over a realized tick trace of batches. -/
def tfoldHist {α σ : Type} (g : σ → α → σ) (init : σ) (bs : Trace (Array α)) : Trace σ :=
  scanAcrossTicksTrace (tfoldStep g) init bs

/-- `tfoldHist` is the cross-tick fold of the batches. -/
theorem tfoldHist_eq_fold {α σ : Type} (g : σ → α → σ) :
    ∀ (init : σ) (bs : Trace (Array α)),
      tfoldHist g init bs = foldAcrossTicksTrace (fun s b => b.foldl g s) init bs
  | _, [] => rfl
  | init, b :: bs => by
    show (b.foldl g init) :: tfoldHist g (b.foldl g init) bs = (b.foldl g init) :: _
    rw [tfoldHist_eq_fold]

/-- `Optional::latest()` at the machine: the last update of the tick wire's realized trace,
read off its frozen step history. The wire is step-monotone under `wf`, so the freeze is the
raw trace. -/
def lastUpdateHist {n : Nat} {σ : Type} [DecidableEq σ] (t : Fin n → Nat → Trace (Option σ)) : Fin n → Nat → Option σ :=
  fun i step => lastUpdate (((famHist (fun s j => t j s)) i).view step)

/-! ## The instance -/

/-! ### Tick shapes at the machine

Per-step slicing of a shaped input into this tick's batch-valued tuples, and back. A batch at
the machine is an `Array`: one buffer per pass and no cons cell per element, the carrier the
compiled program runs on. The proofs read it through `Array.toList`. -/

namespace SchedTick
variable {L : Type} {mem : L → Nat}

abbrev Tk (mem : L → Nat) (ℓ : L) (τ : Type) : Type := Fin (mem ℓ) → Nat → Trace τ
abbrev TS (mem : L → Nat) (ℓ : L) (α : Type) [DecidableEq α] (_ord : StrOrd)
    (_ret : Retries) : Type := Fin (mem ℓ) → Nat → Trace (Array α)
abbrev TKS (mem : L → Nat) (ℓ : L) (K V : Type) [DecidableEq K] [DecidableEq V] :
    Type := Fin (mem ℓ) → Nat → Trace (SchedKMap K V)
abbrev TKStr (mem : L → Nat) (ℓ : L) (K V : Type) [DecidableEq K] [DecidableEq V]
    (_ord : StrOrd) (_ret : Retries) : Type := Fin (mem ℓ) → Nat → Trace (Array (K × V))
abbrev BS (σ : Type) : Type := σ
abbrev BStr (α : Type) [DecidableEq α] (_ord : StrOrd) (_ret : Retries) : Type := Array α
abbrev BKS (K V : Type) [DecidableEq K] [DecidableEq V] : Type := SchedKMap K V
abbrev BKStr (K V : Type) [DecidableEq K] [DecidableEq V] (_ord : StrOrd) (_ret : Retries) :
    Type := Array (K × V)

/-- A shaped input, per member and step, as one trace of per-tick tuples. -/
def slice {ℓ : L} : (sh : TickShape) → AtomicOf (Tk mem) (TS mem) (TKS mem) (TKStr mem) ℓ sh →
    Fin (mem ℓ) → Nat → Trace (BoundedOf BS BStr BKS BKStr sh)
  | .sing _, x, i, k => x i k
  | .stream _ _ _ _, x, i, k => x i k
  | .ksing _ _ _ _, x, i, k => x i k
  | .kstream _ _ _ _ _ _, x, i, k => x i k
  | .pair a b, x, i, k => Trace.zip (slice a x.1 i k) (slice b x.2 i k)

/-- The inverse of `slice`: a trace of per-tick tuples split back into a shaped output. -/
def unslice {ℓ : L} : (sh : TickShape) →
    (Fin (mem ℓ) → Nat → Trace (BoundedOf BS BStr BKS BKStr sh)) →
    AtomicOf (Tk mem) (TS mem) (TKS mem) (TKStr mem) ℓ sh
  | .sing _, f => f
  | .stream _ _ _ _, f => f
  | .ksing _ _ _ _, f => f
  | .kstream _ _ _ _ _ _, f => f
  | .pair a b, f =>
    (unslice a (fun i k => (f i k).map Prod.fst),
     unslice b (fun i k => (f i k).map Prod.snd))

/-- The initial register of a shape from its seed; a stream register starts empty. -/
def seed : (sh : TickShape) → SeedOf sh → BoundedOf BS BStr BKS BKStr sh
  | .sing _, v => v
  | .stream _ _ _ _, _ => #[]
  | .ksing _ _ _ _, _ => SchedKMap.empty
  | .kstream _ _ _ _ _ _, _ => #[]
  | .pair a b, s => (seed a s.1, seed b s.2)

/-- The tick former at the machine: the structural scan of `g` over the sliced input, on the
runtime's batches, as of each step. -/
def scan {ℓ : L} (sts ins outs : TickShape)
    (x : AtomicOf (Tk mem) (TS mem) (TKS mem) (TKStr mem) ℓ ins)
    (g : Fin (mem ℓ) → BoundedOf BS BStr BKS BKStr sts → BoundedOf BS BStr BKS BKStr ins →
      BoundedOf BS BStr BKS BKStr sts × BoundedOf BS BStr BKS BKStr outs)
    (init : SeedOf sts) : AtomicOf (Tk mem) (TS mem) (TKS mem) (TKStr mem) ℓ outs :=
  unslice outs (fun i k => scanAcrossTicksTrace (g i) (seed sts init) (slice ins x i k))

end SchedTick

set_option warn.classDefReducibility false in
/-- The step machine over a pacing `pacing`: what each member of each location does at each
step (idle, stutter or tick), independently across cluster members. -/
def SchedSem (L : Type) (mem : L → Nat) (pacing : Pacing L mem) :
    HydroSem L mem where
  Stream ℓ α _ _ _ := Fin (mem ℓ) → StepHist α
  -- a keyed stream is represented by the runtime's flat wire of pairs (`into_keyed` is a cast; the
  -- network's pairs are tagged by sender)
  KeyedStream ℓ K V _ _ _ _ := Fin (mem ℓ) → StepHist (K × V)
  -- a folded singleton is its live value as of each step
  Singleton ℓ α σ _ _ _ _b := Fin (mem ℓ) → Nat → σ
  -- a keyed fold's live map as of each step: the runtime's association list, keys distinct
  KeyedSingleton ℓ _α K V _ _ _ _ _ _b := Fin (mem ℓ) → Nat → SchedKMap K V
  -- the ticked carrier: per member, the realized tick sequence as of each machine step
  -- (which step each tick ran on); a raw per-step view with no monotonicity bundle
  Atomic ℓ σ := Fin (mem ℓ) → Nat → Trace σ
  -- one tick's bounded stream is represented by the concrete batch the runtime holds, an
  -- `Array`, at every grade: the quotient belongs to the denotation, and here order and retries are unobservable only
  -- through the in-tick operators' typing
  BoundedStream α _ _ _ := Array α
  BoundedSingleton σ := σ
  BoundedKeyedStream K V _ _ _ _ := Array (K × V)
  BoundedKeyedSingleton K V _ _ := SchedKMap K V
  -- `AtomicStream` takes the signature default, `Fin (mem ℓ) → Nat → Trace (Array α)`
  TransportDec p c := Fin p → Fin c → Nat → Nat
  OrderSelDec _ _ := Unit
  TickOrderSelDec _ _ := Unit
  SnapDec _ _ _ := Unit
  BatchDec _ _ _ _ := Unit
  -- timing at the machine is a total per-tick script: the pacing decides how many ticks
  -- there are, the script what each tick reads
  IntervalDec n := TickScript n Nat
  TicksDec _ := Unit
  ClockDec n := TickScript n Nat
  FixDec := Unit
  map s f := fun i => (s i).map (f i)
  filterMap s f := fun i => (s i).filterMap (f i)
  broadcast_closed {_c p _α _ _ord _ret} d s := fun i =>
    mergeN (fun j => ((s j).deliver (pacing.active p i) (d i j)).map (fun v => (j, v)))
  demux {_c p _α _ _ord} d s addr := fun i =>
    mergeN (fun j => (((s j).filterMap
      (fun dx => if dx.1 = addr i then some dx.2 else none)).deliver
      (pacing.active p i) (d i j)).map (fun v => (j, v)))
  values k := fun i => (k i).map Prod.snd
  into_keyed s := s
  kassume_ordering u _d := u
  kfold_early_stop ks f init := fun i t =>
    keyedFoldEarlyStopAssoc f init ((ks i).view t)
  kget_max_key ks := fun i t => (ks i t).maxEntry
  weaken_retries s := s
  union a b := fun i => merge2 (a i) (b i)
  assume_ordering u _d := u
  fold g init _Pre _ _ok s := fun i => schedFold g init (s i)
  fold_monotone _vo g init _ok _hinfl s := fun i => schedFold g init (s i)
  snapshot {ℓ _α _σ _ _ord _ret _b} s _d := fun i t =>
    (tickSteps (pacing.ticks ℓ i) t).map (fun st => s i st)
  batch {ℓ _α _ _ord _ret} s _d := fun i t =>
    (batchesFrom ((s i).view) (tickSteps (pacing.ticks ℓ i) t) 0).map List.toArray
  source_interval {ℓ} _delay _period d := fun i => intervalHist (pacing.ticks ℓ i) (d i)
  firstTick t := fun i step => (t i step).map (fun b => b[0]?)
  mapTick s f := fun i step => (s i step).map (f i)
  zipTick a b := fun i step => Trace.zip (a i step) (b i step)
  -- DFIR's `defer_tick` (`dfir_lang/src/graph/ops/defer_tick.rs:6`, "Buffers all input items
  -- and releases them at the next time boundary"; emitted for `HydroNode::DeferTick` at
  -- `hydro_lang/src/compile/ir/mod.rs:5938–5961` as `defer_tick_lazy()`). The buffer is
  -- `init :: t` beyond what has been released, and a tick releases its head, so entry `j` of
  -- the output is `(init :: t)[j]`: tick `0` reads the seed, tick `j` the input's entry for
  -- tick `j-1`. For an input at most one tick behind the tick count this is
  -- `(init :: t).take ticks` (`deferReleased_eq_ticks`). Release happens before push: a tick
  -- releases from the buffer as of the previous step (`init :: t i (step - 1)`, the released
  -- count reading the input's length one step back), so what the input pushes at a tick is
  -- released at the next tick, as in DFIR. `defer_tick` is therefore strict in its input, and
  -- it is what closes a cycle through a tick (the knot wire itself reads the body's Kleene
  -- stage at the same step).
  defer_tick {ℓ _σ} init t := fun i step =>
    (init :: t i (step - 1)).take
      (deferReleased (pacing.ticks ℓ i) (fun k => (t i (k - 1)).length) step)
  allTicks bs := famHist (fun step i => ((bs i step).map Array.toList).flatten)
  flattenOrdered t := fun i step => (t i step).map List.toArray
  flattenUnordered t := fun i step => (t i step).map List.toArray
  -- the knot wire at step `t` reads stage `t + 1` of the body's Kleene iteration at `t` (the
  -- Kleene diagonal, ungated): a
  -- `forward_ref` that is acyclic through strata is a same-tick binding in DFIR. The diagonal
  -- is a fixpoint of the body when the body is strict in the knot wire — every path from the
  -- wire to the knot's output passes a network hop or a `defer_tick`, which read one step
  -- back — and the generated knot `wf` proves that (`fix_stream_eq`/`fix_tick_eq` in
  -- `Hydro/SchedCausal.lean`). The stream knot re-monotonizes through `famHist`; the tick
  -- diagonal sits behind `stepGate` (inert for the kernel at a symbolic step;
  -- `fix_tick_apply`).
  fix_stream {ℓ _α _ _ord _ret} _d body :=
    famHist (fun t i => ((iterate body (fun _ => StepHist.bot) (t + 1)) i).view t)
  fix_tick {_ℓ _σ} _d body := fun i t =>
    stepGate (fun t => (iterate body (fun _ _ => []) (t + 1)) i t) t
  tinto_keyed x := x
  tassume_ordering u _d := u
  -- DFIR's `ReduceKeyedWatermark` (`compile/ir/mod.rs:6389–6576`): a `fold` over
  -- `(map, watermark)` fed this tick's payloads then its watermark, with eager GC
  -- (`map.retain(|k, _| *k >= watermark)`) — on the association list
  reduce_watermark ks wm f _Pre _ _ok := fun i t =>
    scanAcrossTicksTrace (reduceWatermarkEagerH f) (SchedKMap.empty, none)
      (Trace.zip ((ks i t).map Array.toList) (wm i t))
  -- the keyed-stream and keyed-singleton vocabulary
  kentries ks := ks
  kmap ks f := fun i => (ks i).map (fun e => (e.1, f i e.2))
  kallTicks bs := famHist (fun step i => ((bs i step).map Array.toList).flatten)
  kfold g init _Pre _ _ok ks := fun i t => keyedFoldAssocH g init ((ks i).view t)
  ksnapshot {ℓ _α _K _V _ _ _ _ord _ret _b} ks _d := fun i t =>
    (tickSteps (pacing.ticks ℓ i) t).map (fun st => ks i st)
  tfold g init _Pre _ _ok bs := fun i t => tfoldHist g init (bs i t)
  latest t := lastUpdateHist t
  latest_monotone t := lastUpdateHist t
  time {ℓ} d := fun i step => scriptTrace (d i) (tickSteps (pacing.ticks ℓ i) step).length
  snapshot_bounded {ℓ _σ} x _d := fun i step =>
    List.replicate (tickSteps (pacing.ticks ℓ i) step).length x
  -- in-tick operators on the runtime's batches, at every grade; the hash-backed ones are the
  -- array forms of `Hydro/KeyedHash.lean`, specified through `toList`
  bmap b f := b.map f
  bfilterMap b f := b.filterMap f
  bflatMapOrdered l f := l.flatMap (fun a => (f a).toArray)
  bflatMapUnordered b f := b.flatMap f
  bofList l := l.toArray
  bcount b := b.size
  bisEmpty b := b.isEmpty
  bfold g init _Pre _ _ok b := b.foldl g init
  benumerate l := (listEnumerate l.toList).toArray
  bfirst l := l[0]?
  bcrossSingleton b s := b.map (fun a => (a, s))
  bchain a b := a ++ b
  bweakenOrder b := b
  bfilter b p := b.filter p
  binto_keyed b := b
  bkfold g init _Pre _ _ok ks := keyedFoldArrH g init ks
  bksFilter m p := m.filter p
  bksMap m f := m.map f
  bksKeys m := m.keys.toArray
  bksEntries m := m.entries.toArray
  bjoin a b := arrJoinH a b
  bantiJoin a ks := arrAntiJoin a ks
  bfilterNotIn a o := arrFilterNotIn a o
  bmax b := b.foldl maxStep none
  bfilterIf b flag := if flag then b else #[]
  bsPure v := v
  bsMap s f := f s
  bsZip a b := (a, b)
  boMap o f := o.map f
  boUnwrapOr o s := o.getD s
  boOr o o' := o.or o'
  boFilterIf o b := if b then o else none
  boFilter o p := o.filter p
  boIsSome o := o.isSome
  boIsNone o := o.isNone
  boZip a b := a.bind (fun x => b.map (fun y => (x, y)))
  boFilterMap o f := o.bind f
  boIntoStream o := o.toArray
  bsFilterIf s flag := if flag then some s else none
  bsFilterMap s f := f s
  boZipSingleton o s := o.map (fun x => (x, s))
  bsZipOptional s o := o.map (fun y => (s, y))
  bmin b := b.foldl minStep none
  breduce g _Pre _ _ok b := b.foldl (reduceStep g) none
  bsort b := b.mergeSort (fun a b => decide (a ≤ b))
  bcrossOptional b o := match o with | none => #[] | some s => b.map (fun a => (a, s))
  bkmap ks f := ks.map (fun e => (e.1, f e.2))
  bkcrossSingleton ks s := ks.map (fun e => (e.1, (e.2, s)))
  bkchain a b := a ++ b
  bkweakenOrder ks := ks
  -- the `H` forms: the same lists, hash-backed in compiled code (`Hydro/KeyedHash.lean`)
  bkreduce g _Pre _ _ok ks := keyedReduceAssocH g ks.toList
  bksJoin a b := a.joinH b
  bksFilterKeyNotIn m ks := m.filterKeyNotInH ks
  bksKeyCount m := m.keyCount
  bksIntoStream m := m.entries.toArray
  tick_scan sts ins outs x g init := SchedTick.scan sts ins outs x g init

/-- The tick knot applied at a step is the raw Kleene diagonal: the gate is an identity operator. -/
theorem fix_tick_apply {L : Type} {mem : L → Nat} {pacing : Pacing L mem} {ℓ : L} {σ : Type}
    (d : Unit) (body : (SchedSem L mem pacing).Atomic ℓ σ → (SchedSem L mem pacing).Atomic ℓ σ)
    (i : Fin (mem ℓ)) (t : Nat) :
    (SchedSem L mem pacing).fix_tick d body i t = (iterate body (fun _ _ => []) (t + 1)) i t :=
  stepGate_eq _ t

/-- The tick knot is the raw diagonal, as a function. -/
theorem fix_tick_eq_diag {L : Type} {mem : L → Nat} {pacing : Pacing L mem} {ℓ : L} {σ : Type}
    (d : Unit) (body : (SchedSem L mem pacing).Atomic ℓ σ → (SchedSem L mem pacing).Atomic ℓ σ) :
    (SchedSem L mem pacing).fix_tick d body
      = fun i t => (iterate body (fun _ _ => []) (t + 1)) i t := by
  funext i t; exact fix_tick_apply d body i t

end Hydro
