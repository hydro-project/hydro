import Hydro.Project.Model.DistributedModel
import Hydro.Project.Model.SweepSpec
import Mathlib.Data.List.Dedup

/-!
# Hydro · Project · Model — a run at the sockets is a run of the model

**Model, not production.** What a deployment produces is a **trace at the bytes**: a sequence
of activations, each saying which process ran, the inputs it was given, the clock and timer
readings it took, and how many bytes it read off each of its connections (`WireTrace`).
The rest follows from TCP and the process's body: a connection is a byte queue (TCP), a process
reads some prefix of what is queued, cut anywhere, mid-frame included, and runs the production
body `Dag.iterateBytes` on it; the complete frames are its messages, the incomplete tail is kept
for the next activation, and its sends go on its connections as frames (`WireTrace.run`). There
is no assumption to state: every `who`/`inp`/`readings`/`reads` is a trace. What stays outside
is crash-stop (a process that stops has no further activations) and the sockets themselves.

This file shows every such trace is a batched interleaving. The proof goes through
`PassTrace`, the same trace with a tick flag per step: on a valid graph a wire trace runs as
the `PassTrace` that ticks at every step (`WireTrace.toPass`, by
`Dag.iterateBytes_eq_passBytes`), and every `PassTrace` induces an interleaving
(`PassTrace.interleaving`, `PassTrace.batched`) in which the delivery cursor of a connection is
the number of its messages among the frames the receiver has parsed (`traceSched`), the timing
scripts are the readings indexed by tick, and the trace's run matches the model's run at the
wire (`PassTrace.act_eq`). `WireTrace.streamOutputsAt_eq` states the result for the wire trace;
`paxos_safe_wired` (`Paxos/WiredSafety.lean`) is then a headline with no interleaving in its
statement: it quantifies over traces.

Two invariants carry the induction. On the run itself (`RunInv`): a receiver's byte cursor on
a connection is within what the sender has written, and its kept tail is the bytes past the
last complete frame (`Wire.splitFrames` says where that is). On the lock-step cells
(`Dag.TraceQueues`): each receiving end's queue holds the connection's messages past the
parsed prefix, in order, and its delivered count is the message count of the parsed prefix;
each timer cell is the member's tick count. The framing law
(`Wire.framesFrom_prefix_frameAll`) turns bytes read into messages parsed: cutting a stream of
frames anywhere yields the frames that fit and keeps the rest.
-/

namespace Hydro

namespace Project

variable {L : Type} {mem : L → Nat} [DecidableEq L]

/-! ## The trace's combinatorics -/

/-- The activations of member `m` before step `K`, in order. -/
def activationsOf (who : Nat → Member L mem) (m : Member L mem) (K : Nat) : List Nat :=
  (List.range K).filter fun k => decide (who k = m)

/-- Step `K` is appended iff it is `m`'s. -/
theorem activationsOf_succ (who : Nat → Member L mem) (m : Member L mem) (K : Nat) :
    activationsOf who m (K + 1) = activationsOf who m K ++ (if who K = m then [K] else []) := by
  unfold activationsOf
  rw [List.range_succ, List.filter_append, List.filter_singleton]
  split <;> simp_all

/-- The activation list is prefix-monotone in `K`. -/
theorem activationsOf_mono (who : Nat → Member L mem) (m : Member L mem) {K K' : Nat} (h : K ≤ K') :
    activationsOf who m K <+: activationsOf who m K' := by
  induction K' with
  | zero => cases Nat.le_zero.mp h; exact List.prefix_refl _
  | succ K' ih =>
    rcases Nat.lt_or_ge K (K' + 1) with hlt | hge
    · exact (ih (Nat.lt_succ_iff.mp hlt)).trans (by rw [activationsOf_succ]; exact List.prefix_append _ _)
    · cases Nat.le_antisymm h hge; exact List.prefix_refl _

/-- Membership in the activation list: before `K` and `m`'s. -/
theorem mem_activationsOf {who : Nat → Member L mem} {m : Member L mem} {K k : Nat} :
    k ∈ activationsOf who m K ↔ k < K ∧ who k = m := by
  simp [activationsOf]

/-- Activation counts separate activations: two activations of `m` with the same count are the
same step, because the count strictly grows at each of `m`'s activations. -/
theorem activationsOf_length_inj {who : Nat → Member L mem} {m : Member L mem} {k k' : Nat}
    (hk : who k = m) (hk' : who k' = m)
    (h : (activationsOf who m k).length = (activationsOf who m k').length) : k = k' := by
  by_contra hne
  rcases Nat.lt_or_gt_of_ne hne with hlt | hlt
  · have h1 : activationsOf who m (k + 1) <+: activationsOf who m k' := activationsOf_mono who m hlt
    have h2 := h1.length_le
    rw [activationsOf_succ, if_pos hk, List.length_append] at h2
    simp at h2; omega
  · have h1 : activationsOf who m (k' + 1) <+: activationsOf who m k := activationsOf_mono who m hlt
    have h2 := h1.length_le
    rw [activationsOf_succ, if_pos hk', List.length_append] at h2
    simp at h2; omega

open Classical in
/-- The `n`-th activation of member `m`, if it has happened, with its witness. Noncomputable:
it chooses the step by `activationsOf_length_inj`. -/
noncomputable def nthActivation (who : Nat → Member L mem) (m : Member L mem) (n : Nat) :
    Option {k : Nat // who k = m} :=
  if h : ∃ k, who k = m ∧ (activationsOf who m k).length = n then
    some ⟨Classical.choose h, (Classical.choose_spec h).1⟩
  else none

/-- `nthActivation` at an activation's own count is that activation. -/
theorem nthActivation_count (who : Nat → Member L mem) (k : Nat) :
    nthActivation who (who k) (activationsOf who (who k) k).length = some ⟨k, rfl⟩ := by
  unfold nthActivation
  have h : ∃ k', who k' = who k ∧ (activationsOf who (who k) k').length = (activationsOf who (who k) k).length :=
    ⟨k, rfl, rfl⟩
  rw [dif_pos h]
  have hs := Classical.choose_spec h
  have : Classical.choose h = k := activationsOf_length_inj hs.1 rfl hs.2
  congr 1
  exact Subtype.ext this

/-- `nthActivation_count` with the member given by `h`. -/
theorem nthActivation_count' (who : Nat → Member L mem) (k : Nat) {m : Member L mem} (h : who k = m) :
    nthActivation who m (activationsOf who m k).length = some ⟨k, h⟩ := by
  subst h
  exact nthActivation_count who k

/-- What `src` has put on its connection to `m` by step `K`: its activations' messages for `m`,
in order, where `sent k' m` is the messages activation `k'` put on that connection. -/
def streamOf (who : Nat → Member L mem) (sent : Nat → Member L mem → List WireMsg) (m src : Member L mem) (K : Nat) :
    List WireMsg :=
  (activationsOf who src K).flatMap fun k' => sent k' m

/-- Step `K`'s messages are appended iff it is `src`'s activation. -/
theorem streamOf_succ (who : Nat → Member L mem) (sent : Nat → Member L mem → List WireMsg) (m src : Member L mem)
    (K : Nat) :
    streamOf who sent m src (K + 1) = streamOf who sent m src K ++ (if who K = src then sent K m else []) := by
  unfold streamOf
  rw [activationsOf_succ, List.flatMap_append]
  split <;> simp

/-- The bytes requested from connection `src` this activation: the sum of its entries in
`reads` (connection, byte count); a connection listed twice is read twice. -/
def bytesReq (reads : List (Member L mem × Nat)) (src : Member L mem) : Nat :=
  ((reads.filter fun r => decide (r.1 = src)).map Prod.snd).sum

/-- The connections read this activation, each once. -/
def srcsOf (reads : List (Member L mem × Nat)) : List (Member L mem) := (reads.map Prod.fst).dedup

/-- `srcsOf` has no duplicates. -/
theorem srcsOf_nodup (reads : List (Member L mem × Nat)) : (srcsOf reads).Nodup := List.nodup_dedup _

/-- A connection not read requests no bytes. -/
theorem bytesReq_eq_zero {reads : List (Member L mem × Nat)} {src : Member L mem} (h : src ∉ srcsOf reads) :
    bytesReq reads src = 0 := by
  unfold bytesReq
  rw [List.sum_eq_zero]
  intro x hx
  obtain ⟨r, hr, rfl⟩ := List.mem_map.mp hx
  have := List.mem_filter.mp hr
  exact absurd (List.mem_dedup.mpr (List.mem_map.mpr ⟨r, this.1, of_decide_eq_true this.2⟩)) h


/-! ### Ticks

A `PassTrace` has ticks and stutters; a member's timer cells count its ticks, so the induced
scripts index by tick count. -/

/-- The ticks of member `m` before step `K`, in order. -/
def ticksOf (who : Nat → Member L mem) (tick : Nat → Bool) (m : Member L mem) (K : Nat) : List Nat :=
  (List.range K).filter fun k => decide (who k = m) && tick k

/-- Step `K` is appended iff it is `m`'s and ticks. -/
theorem ticksOf_succ (who : Nat → Member L mem) (tick : Nat → Bool) (m : Member L mem) (K : Nat) :
    ticksOf who tick m (K + 1) = ticksOf who tick m K ++ (if who K = m ∧ tick K = true then [K] else []) := by
  unfold ticksOf
  rw [List.range_succ, List.filter_append, List.filter_singleton]
  by_cases h : who K = m ∧ tick K = true
  · rw [if_pos h]; simp [h.1, h.2]
  · rw [if_neg h]
    have : (decide (who K = m) && tick K) = false := by
      cases hw : decide (who K = m) <;> cases ht : tick K <;> simp_all
    simp [this]

/-- The tick list is prefix-monotone in `K`. -/
theorem ticksOf_mono (who : Nat → Member L mem) (tick : Nat → Bool) (m : Member L mem) {K K' : Nat} (h : K ≤ K') :
    ticksOf who tick m K <+: ticksOf who tick m K' := by
  induction K' with
  | zero => cases Nat.le_zero.mp h; exact List.prefix_refl _
  | succ K' ih =>
    rcases Nat.lt_or_ge K (K' + 1) with hlt | hge
    · exact (ih (Nat.lt_succ_iff.mp hlt)).trans (by rw [ticksOf_succ]; exact List.prefix_append _ _)
    · cases Nat.le_antisymm h hge; exact List.prefix_refl _

/-- Membership in the tick list: before `K`, `m`'s, and a tick. -/
theorem mem_ticksOf {who : Nat → Member L mem} {tick : Nat → Bool} {m : Member L mem} {K k : Nat} :
    k ∈ ticksOf who tick m K ↔ k < K ∧ who k = m ∧ tick k = true := by
  simp [ticksOf]

/-- Tick counts separate ticks: two ticks of `m` with the same count are the same step. -/
theorem ticksOf_length_inj {who : Nat → Member L mem} {tick : Nat → Bool} {m : Member L mem} {k k' : Nat}
    (hk : who k = m) (htk : tick k = true) (hk' : who k' = m) (htk' : tick k' = true)
    (h : (ticksOf who tick m k).length = (ticksOf who tick m k').length) : k = k' := by
  by_contra hne
  rcases Nat.lt_or_gt_of_ne hne with hlt | hlt
  · have h1 : ticksOf who tick m (k + 1) <+: ticksOf who tick m k' := ticksOf_mono who tick m hlt
    have h2 := h1.length_le
    rw [ticksOf_succ, if_pos ⟨hk, htk⟩, List.length_append] at h2
    simp at h2; omega
  · have h1 : ticksOf who tick m (k' + 1) <+: ticksOf who tick m k := ticksOf_mono who tick m hlt
    have h2 := h1.length_le
    rw [ticksOf_succ, if_pos ⟨hk', htk'⟩, List.length_append] at h2
    simp at h2; omega

open Classical in
/-- The `n`-th tick of member `m`, if it has happened, with its witness. Noncomputable: it
chooses the step by `ticksOf_length_inj`. -/
noncomputable def nthTick (who : Nat → Member L mem) (tick : Nat → Bool) (m : Member L mem) (n : Nat) :
    Option {k : Nat // who k = m} :=
  if h : ∃ k, who k = m ∧ tick k = true ∧ (ticksOf who tick m k).length = n then
    some ⟨Classical.choose h, (Classical.choose_spec h).1⟩
  else none

/-- `nthTick` at a tick's own count is that tick. -/
theorem nthTick_count (who : Nat → Member L mem) (tick : Nat → Bool) (k : Nat) (htk : tick k = true) :
    nthTick who tick (who k) (ticksOf who tick (who k) k).length = some ⟨k, rfl⟩ := by
  unfold nthTick
  have h : ∃ k', who k' = who k ∧ tick k' = true ∧ (ticksOf who tick (who k) k').length = (ticksOf who tick (who k) k).length :=
    ⟨k, rfl, htk, rfl⟩
  rw [dif_pos h]
  have hs := Classical.choose_spec h
  have : Classical.choose h = k := ticksOf_length_inj hs.1 hs.2.1 rfl htk hs.2.2
  congr 1
  exact Subtype.ext this

/-- `nthTick_count` with the member given by `h`. -/
theorem nthTick_count' (who : Nat → Member L mem) (tick : Nat → Bool) (k : Nat) (htk : tick k = true) {m : Member L mem}
    (h : who k = m) : nthTick who tick m (ticksOf who tick m k).length = some ⟨k, h⟩ := by
  subst h
  exact nthTick_count who tick k htk

/-! ## The pass-level trace (the proof layer's object)

`PassTrace` models a run as one pass, a tick or a stutter, per step. The
trusted object, a `WireTrace` with one activation per step and each activation the production
body `Dag.iterateBytes` (one tick), is defined below (`## The trace`); on a valid graph it is
the `PassTrace` that ticks at every step (`WireTrace.toPass`), and every `PassTrace` is a
batched interleaving. -/

/-- **A trace at the bytes with a tick flag per step**: who ran at each step, whether the pass
ticks, the inputs it was given, the clock and timer readings it took, and how many bytes it
read off each of its connections. -/
structure PassTrace {Γ : Ctx L mem} {outs : List (Kind L mem)} (d : Dag Γ outs) where
  who : Nat → Member L mem
  /-- Whether the pass ticks; a stutter runs no tick body and reads no timer. -/
  tick : Nat → Bool
  inp : Nat → HList (Kind.delta (mem := mem)) Γ
  readings : (k : Nat) → TickReadings (who k)
  reads : Nat → List (Member L mem × Nat)

/-- The run so far: every process's cells, per connection the receiver's byte cursor and kept
tail, and what each earlier activation put on the wire. -/
structure RunState {Γ : Ctx L mem} {outs : List (Kind L mem)} (d : Dag Γ outs) where
  cells : (m : Member L mem) → Dag.LocState m d
  cur : Member L mem → Member L mem → Nat
  buf : Member L mem → Member L mem → ByteArray
  sends : Nat → List WireMsg

namespace PassTrace

variable {Γ : Ctx L mem} {outs : List (Kind L mem)} {d : Dag Γ outs}

/-- A process's cells before its first activation: the initial cells cut to it, with no queue. -/
def initCells (m : Member L mem) : Dag.LocState m d := Dag.arrivedOnly m d (Dag.restrict m d (Dag.init d))

/-- The bytes `src` has put on its connection to `m` by step `k`, given the activations' sends
so far: the stream of its messages for `m`, framed. -/
def bytesOn (who : Nat → Member L mem) (sends : Nat → List WireMsg) (m src : Member L mem) (k : Nat) : ByteArray :=
  frameMsgs (streamOf who (fun k' m' => Dag.wireFor m' d (sends k')) m src k)

/-- Activation `k` on a run state: `Dag.passBytes` on the running process's cells, handed each
listed connection's kept tail plus the requested bytes, as many as are there. -/
def actOf (cod : Dag.Codecs d) (τ : PassTrace d) (k : Nat) (r : RunState d) :=
  let m := τ.who k
  let S := fun src => bytesOn (d := d) τ.who r.sends m src k
  let cur' := fun src => r.cur m src + min (bytesReq (τ.reads k) src) ((S src).size - r.cur m src)
  let bytes := fun src => r.buf m src ++ (S src).extract (r.cur m src) (cur' src)
  let chans := (srcsOf (τ.reads k)).map fun src => (⟨src, bytes src⟩ : (src : Member L mem) × ByteArray)
  Dag.passBytes m (τ.tick k) d cod (Dag.Timing.ofReadings m (τ.readings k) d) chans (envRestrict m (τ.inp k)) (r.cells m)

/-- Step `k` of the run: the activation's cells replace the running process's, its cursors
advance and its tails are the unparsed remainders on the connections it read, and its sends go
on the wire. -/
def step (cod : Dag.Codecs d) (τ : PassTrace d) (k : Nat) (r : RunState d) : RunState d :=
  let m := τ.who k
  let S := fun src => bytesOn (d := d) τ.who r.sends m src k
  let cur' := fun src => r.cur m src + min (bytesReq (τ.reads k) src) ((S src).size - r.cur m src)
  let bytes := fun src => r.buf m src ++ (S src).extract (r.cur m src) (cur' src)
  let a := τ.actOf cod k r
  { cells := fun m' => if h : m = m' then h ▸ a.1 else r.cells m'
    cur := fun m' src => if m = m' then cur' src else r.cur m' src
    buf := fun m' src => if m = m' ∧ src ∈ srcsOf (τ.reads k) then (parseFrames (bytes src)).2 else r.buf m' src
    sends := fun k' => if k' = k then a.2.2.1 else r.sends k' }

/-- The run: `initCells` everywhere, cursors at `0`, empty tails, nothing sent; then `step`. -/
def run (cod : Dag.Codecs d) (τ : PassTrace d) : Nat → RunState d
  | 0 => ⟨fun m => initCells m, fun _ _ => 0, fun _ _ => .empty, fun _ => []⟩
  | k + 1 => τ.step cod k (τ.run cod k)

/-- Process `m`'s cells before step `k`. -/
def cells (cod : Dag.Codecs d) (τ : PassTrace d) (k : Nat) (m : Member L mem) : Dag.LocState m d := (τ.run cod k).cells m

/-- The messages activation `k` put on the wire. -/
def sends (cod : Dag.Codecs d) (τ : PassTrace d) (k : Nat) : List WireMsg := (τ.run cod (k + 1)).sends k

/-- What activation `k'` put on its connection to `m`. -/
def sent (cod : Dag.Codecs d) (τ : PassTrace d) (k' : Nat) (m : Member L mem) : List WireMsg :=
  Dag.wireFor m d (τ.sends cod k')

/-- The messages `src` has put on its connection to `m` by step `k`. -/
def stream (cod : Dag.Codecs d) (τ : PassTrace d) (m src : Member L mem) (k : Nat) : List WireMsg :=
  streamOf τ.who (τ.sent cod) m src k

/-- Receiver `m`'s byte cursor on its connection from `src` before step `k`. -/
def cur (cod : Dag.Codecs d) (τ : PassTrace d) (k : Nat) (m src : Member L mem) : Nat := (τ.run cod k).cur m src

/-- The tail `m` keeps from its connection from `src` before step `k`. -/
def buf (cod : Dag.Codecs d) (τ : PassTrace d) (k : Nat) (m src : Member L mem) : ByteArray := (τ.run cod k).buf m src

/-- How many of the connection's messages `m` has parsed from `src` before step `k`: the frames
that fit in the bytes read (`Wire.splitFrames`). -/
def pc (cod : Dag.Codecs d) (τ : PassTrace d) (k : Nat) (m src : Member L mem) : Nat :=
  (Wire.splitFrames (τ.stream cod m src k) (τ.cur cod k m src)).1

/-- The byte cursor after step `k` on the running process's connection from `src`: advanced by
the bytes requested, capped at what is on the wire. -/
def cur' (cod : Dag.Codecs d) (τ : PassTrace d) (k : Nat) (src : Member L mem) : Nat :=
  τ.cur cod k (τ.who k) src
    + min (bytesReq (τ.reads k) src) ((frameMsgs (τ.stream cod (τ.who k) src k)).size - τ.cur cod k (τ.who k) src)

/-- The bytes the running process hands the body for its connection from `src` at step `k`: the
kept tail plus the newly read bytes. -/
def chanBytes (cod : Dag.Codecs d) (τ : PassTrace d) (k : Nat) (src : Member L mem) : ByteArray :=
  τ.buf cod k (τ.who k) src ++ (frameMsgs (τ.stream cod (τ.who k) src k)).extract (τ.cur cod k (τ.who k) src) (τ.cur' cod k src)

/-- The per-connection bytes handed the body at step `k`, one entry per connection read. -/
def chans (cod : Dag.Codecs d) (τ : PassTrace d) (k : Nat) : List ((src : Member L mem) × ByteArray) :=
  (srcsOf (τ.reads k)).map fun src => ⟨src, τ.chanBytes cod k src⟩

/-- Activation `k`: `Dag.passBytes` on the running process's cells, stated through `chans` and
`cells` (`act_eq_actOf` relates it to the run's `actOf`). -/
def act (cod : Dag.Codecs d) (τ : PassTrace d) (k : Nat) :=
  Dag.passBytes (τ.who k) (τ.tick k) d cod (Dag.Timing.ofReadings (τ.who k) (τ.readings k) d) (τ.chans cod k)
    (envRestrict (τ.who k) (τ.inp k)) (τ.cells cod k (τ.who k))

/-- The messages activation `k` parsed off its connection from `src`: the complete frames in
`chanBytes`. -/
def batches (cod : Dag.Codecs d) (τ : PassTrace d) (k : Nat) : List ((src : Member L mem) × List WireMsg) :=
  (srcsOf (τ.reads k)).map fun src => ⟨src, (parseFrames (τ.chanBytes cod k src)).1⟩

/-- An earlier activation's sends are fixed in every later run state. -/
theorem run_sends_stable (cod : Dag.Codecs d) (τ : PassTrace d) {k' : Nat} :
    ∀ {k}, k' < k → (τ.run cod k).sends k' = τ.sends cod k'
  | 0, h => absurd h (Nat.not_lt_zero _)
  | k + 1, h => by
    rcases Nat.lt_or_eq_of_le (Nat.lt_succ_iff.mp h) with hlt | rfl
    · show (if k' = k then _ else (τ.run cod k).sends k') = _
      rw [if_neg (Nat.ne_of_lt hlt)]
      exact run_sends_stable cod τ hlt
    · rfl

/-- The bytes the run computes on a connection are the trace's stream, framed. -/
theorem bytesOn_eq (cod : Dag.Codecs d) (τ : PassTrace d) (k : Nat) (m src : Member L mem) :
    bytesOn (d := d) τ.who (τ.run cod k).sends m src k = frameMsgs (τ.stream cod m src k) := by
  unfold bytesOn stream streamOf sent frameMsgs
  congr 1
  apply List.flatMap_congr
  intro k' hk'
  show Dag.wireFor m d ((τ.run cod k).sends k') = _
  rw [run_sends_stable cod τ (mem_activationsOf.mp hk').1]

/-- The running process's cursor after step `k` is `cur'`. -/
theorem cur_eq_step (cod : Dag.Codecs d) (τ : PassTrace d) (k : Nat) (src : Member L mem) :
    (τ.run cod (k + 1)).cur (τ.who k) src = τ.cur' cod k src := by
  show (if τ.who k = τ.who k then _ else _) = _
  rw [if_pos rfl]
  unfold cur' cur
  simp only [bytesOn_eq]

/-- Another process's cursor does not move at step `k`. -/
theorem cur_succ_other (cod : Dag.Codecs d) (τ : PassTrace d) (k : Nat) (m src : Member L mem) (hm : τ.who k ≠ m) :
    τ.cur cod (k + 1) m src = τ.cur cod k m src := by
  show (if τ.who k = m then _ else _) = _
  rw [if_neg hm]
  rfl

/-- Another process's tail does not change at step `k`. -/
theorem buf_succ_other (cod : Dag.Codecs d) (τ : PassTrace d) (k : Nat) (m src : Member L mem) (hm : τ.who k ≠ m) :
    τ.buf cod (k + 1) m src = τ.buf cod k m src := by
  show (if τ.who k = m ∧ _ then _ else _) = _
  rw [if_neg (fun h => hm h.1)]
  rfl

/-- The running process's tail after step `k`: the unparsed remainder on a connection it read,
unchanged on one it did not. -/
theorem buf_succ_self (cod : Dag.Codecs d) (τ : PassTrace d) (k : Nat) (src : Member L mem) :
    τ.buf cod (k + 1) (τ.who k) src
      = if src ∈ srcsOf (τ.reads k) then (parseFrames (τ.chanBytes cod k src)).2 else τ.buf cod k (τ.who k) src := by
  show (if τ.who k = τ.who k ∧ src ∈ srcsOf (τ.reads k) then _ else _) = _
  unfold chanBytes buf cur cur'
  simp only [true_and, bytesOn_eq]
  try rfl

/-- `act` unfolded one level: `Dag.passWire` on the parsed batches, with the unparsed
remainders as the tails. -/
theorem act_eq_wire (cod : Dag.Codecs d) (τ : PassTrace d) (k : Nat) :
    τ.act cod k
      = (let r := Dag.passWire (τ.who k) (τ.tick k) d cod (Dag.Timing.ofReadings (τ.who k) (τ.readings k) d)
          (τ.batches cod k) (envRestrict (τ.who k) (τ.inp k)) (τ.cells cod k (τ.who k))
         (r.1, r.2.1, r.2.2, (τ.chans cod k).map fun c => (parseFrames c.2).2)) := by
  unfold act Dag.passBytes chans batches
  simp only [List.map_map, Function.comp_def]

/-- `act` is the run's `actOf` at the run state before `k`. -/
theorem act_eq_actOf (cod : Dag.Codecs d) (τ : PassTrace d) (k : Nat) : τ.act cod k = τ.actOf cod k (τ.run cod k) := by
  unfold act actOf chans chanBytes cur' cur buf cells
  simp only [bytesOn_eq]
  try rfl

/-- The running process's cells after step `k` are the activation's. -/
theorem cells_succ_self (cod : Dag.Codecs d) (τ : PassTrace d) (k : Nat) :
    τ.cells cod (k + 1) (τ.who k) = (τ.act cod k).1 := by
  unfold cells
  simp only [run, step, dif_pos]
  rw [act_eq_actOf]

/-- Another process's cells do not change at step `k`. -/
theorem cells_succ_other (cod : Dag.Codecs d) (τ : PassTrace d) (k : Nat) (m : Member L mem) (hm : τ.who k ≠ m) :
    τ.cells cod (k + 1) m = τ.cells cod k m := by
  unfold cells
  simp only [run, step, dif_neg hm]

/-- Activation `k`'s sends are the activation's wire output. -/
theorem sends_eq (cod : Dag.Codecs d) (τ : PassTrace d) (k : Nat) : τ.sends cod k = (τ.act cod k).2.2.1 := by
  show (if k = k then _ else _) = _
  rw [if_pos rfl, act_eq_actOf]

/-- The items a stream output `v` of process `m` has carried along the trace by step `T`. -/
def streamOutputsAt (cod : Dag.Codecs d) (τ : PassTrace d) (T : Nat) (m : Member L mem) {ℓ : L} {α : Type} {iα : DecidableEq α} {ord : StrOrd}
    {ret : Retries} (v : Var outs (.stream ℓ α iα ord ret)) (i : Hosted m ℓ) : List α :=
  ((List.range T).filterMap fun k => if h : τ.who k = m then some (h ▸ (τ.act cod k).2.1) else none).flatMap
    fun o => (o.get v i).toList

/-- The run together with member `m`'s output increments so far, in one pass over the steps:
what evaluating a trace costs. `streamOutputsAt_eq_runOutputs` says its outputs are
`streamOutputsAt`. -/
def runOutputs (cod : Dag.Codecs d) (τ : PassTrace d) (m : Member L mem) :
    Nat → RunState d × List (HList (Kind.locDelta m) outs)
  | 0 => (τ.run cod 0, [])
  | k + 1 =>
    let p := τ.runOutputs cod m k
    (τ.step cod k p.1, p.2 ++ if h : τ.who k = m then [h ▸ (τ.actOf cod k p.1).2.1] else [])

/-- `runOutputs`' run state is `run`. -/
theorem runOutputs_fst (cod : Dag.Codecs d) (τ : PassTrace d) (m : Member L mem) :
    ∀ k, (τ.runOutputs cod m k).1 = τ.run cod k
  | 0 => rfl
  | k + 1 => by
    show τ.step cod k (τ.runOutputs cod m k).1 = τ.step cod k (τ.run cod k)
    rw [runOutputs_fst cod τ m k]

/-- `runOutputs`' outputs are `m`'s activations' output increments before `k`. -/
theorem runOutputs_snd (cod : Dag.Codecs d) (τ : PassTrace d) (m : Member L mem) :
    ∀ k, (τ.runOutputs cod m k).2
      = (List.range k).filterMap fun k => if h : τ.who k = m then some (h ▸ (τ.act cod k).2.1) else none
  | 0 => rfl
  | k + 1 => by
    show (τ.runOutputs cod m k).2 ++ (if h : τ.who k = m then [h ▸ (τ.actOf cod k (τ.runOutputs cod m k).1).2.1] else []) = _
    rw [List.range_succ, List.filterMap_append, runOutputs_snd cod τ m k, runOutputs_fst, ← act_eq_actOf]
    congr 1
    by_cases h : τ.who k = m
    · simp [h]
    · simp [h]

/-- `streamOutputsAt` read off `runOutputs`. -/
theorem streamOutputsAt_eq_runOutputs (cod : Dag.Codecs d) (τ : PassTrace d) (T : Nat) (m : Member L mem) {ℓ : L} {α : Type}
    {iα : DecidableEq α} {ord : StrOrd} {ret : Retries} (v : Var outs (.stream ℓ α iα ord ret)) (i : Hosted m ℓ) :
    τ.streamOutputsAt cod T m v i = (τ.runOutputs cod m T).2.flatMap fun o => (o.get v i).toList := by
  unfold streamOutputsAt
  rw [runOutputs_snd]

/-! ### The run invariant: cursors within the stream, tails past the last frame -/

/-- The kept tail a connection should hold at byte cursor `b`: the bytes of the frame cut by
`b`, past the frames that fit. -/
def tailOf (xs : List WireMsg) (b : Nat) : ByteArray :=
  (Wire.frameAll (xs.drop (Wire.splitFrames xs b).1)).extract 0 (Wire.splitFrames xs b).2

/-- The run invariant at step `k`: every byte cursor is within the sender's framed stream, and
every kept tail is `tailOf` at the cursor. -/
structure RunInv (cod : Dag.Codecs d) (τ : PassTrace d) (k : Nat) : Prop where
  cur_le : ∀ m src, τ.cur cod k m src ≤ (frameMsgs (τ.stream cod m src k)).size
  buf_eq : ∀ m src, τ.buf cod k m src = tailOf (τ.stream cod m src k) (τ.cur cod k m src)

/-- The stream grows by activation `k`'s messages iff `k` is the sender's. -/
theorem stream_succ (cod : Dag.Codecs d) (τ : PassTrace d) (m src : Member L mem) (k : Nat) :
    τ.stream cod m src (k + 1) = τ.stream cod m src k ++ (if τ.who k = src then τ.sent cod k m else []) :=
  streamOf_succ τ.who (τ.sent cod) m src k

/-- Framing more messages yields at least as many bytes. -/
theorem size_frameMsgs_le_append (xs ys : List WireMsg) : (frameMsgs xs).size ≤ (frameMsgs (xs ++ ys)).size := by
  unfold frameMsgs
  rw [Wire.frameAll_append, ByteArray.size_append]
  exact Nat.le_add_right _ _

/-- The advanced cursor stays within the stream, because it is capped at the stream's size. -/
theorem cur'_le (cod : Dag.Codecs d) (τ : PassTrace d) (k : Nat) (src : Member L mem)
    (h : τ.cur cod k (τ.who k) src ≤ (frameMsgs (τ.stream cod (τ.who k) src k)).size) :
    τ.cur' cod k src ≤ (frameMsgs (τ.stream cod (τ.who k) src k)).size := by
  unfold cur'
  omega

/-- The cursor never moves back. -/
theorem cur_le_cur' (cod : Dag.Codecs d) (τ : PassTrace d) (k : Nat) (src : Member L mem) :
    τ.cur cod k (τ.who k) src ≤ τ.cur' cod k src := Nat.le_add_right _ _

/-- A connection not read keeps its cursor. -/
theorem cur'_of_not_mem (cod : Dag.Codecs d) (τ : PassTrace d) (k : Nat) (src : Member L mem)
    (h : src ∉ srcsOf (τ.reads k)) : τ.cur' cod k src = τ.cur cod k (τ.who k) src := by
  unfold cur'
  rw [bytesReq_eq_zero h, Nat.zero_min, Nat.add_zero]

/-- Under `RunInv`, the bytes handed the body on a connection are the stream past the parsed
frames, from the cut frame's start up to the new cursor. -/
theorem chanBytes_eq (cod : Dag.Codecs d) (τ : PassTrace d) (k : Nat) (src : Member L mem)
    (hinv : τ.RunInv cod k) :
    τ.chanBytes cod k src
      = (Wire.frameAll ((τ.stream cod (τ.who k) src k).drop (τ.pc cod k (τ.who k) src))).extract 0
          ((Wire.splitFrames (τ.stream cod (τ.who k) src k) (τ.cur cod k (τ.who k) src)).2
            + (τ.cur' cod k src - τ.cur cod k (τ.who k) src)) := by
  unfold chanBytes pc
  rw [hinv.buf_eq, tailOf]
  have hb : τ.cur cod k (τ.who k) src ≤ (Wire.frameAll (τ.stream cod (τ.who k) src k)).size := hinv.cur_le _ _
  have hb' : τ.cur' cod k src ≤ (Wire.frameAll (τ.stream cod (τ.who k) src k)).size := τ.cur'_le cod k src hb
  have hbb' := τ.cur_le_cur' cod k src
  have hspec := Wire.splitFrames_spec _ _ hb
  unfold frameMsgs
  generalize τ.stream cod (τ.who k) src k = xs at *
  generalize τ.cur' cod k src = c at *
  generalize τ.cur cod k (τ.who k) src = b at *
  generalize Wire.splitFrames xs b = sf at *
  have hx : Wire.frameAll xs = Wire.frameAll (xs.take sf.1) ++ Wire.frameAll (xs.drop sf.1) := by
    rw [← Wire.frameAll_append, List.take_append_drop]
  rw [hx, ByteArray.extract_append]
  have h0 : (Wire.frameAll (xs.take sf.1)).extract b c = ByteArray.empty := by
    rw [ByteArray.extract_eq_empty_iff]; omega
  rw [h0, ByteArray.empty_append, show b - (Wire.frameAll (xs.take sf.1)).size = sf.2 by omega,
    ByteArray.extract_append_extract, Nat.zero_min]
  congr 1
  omega

/-- **What activation `k` parses off its connection from `src`**, under `RunInv`: the next
messages of the stream, as many as the bytes read complete, and the new tail is `tailOf` at the
new cursor. The framing law `Wire.framesFrom_prefix_frameAll` does the work. -/
theorem parse_eq (cod : Dag.Codecs d) (τ : PassTrace d) (k : Nat) (src : Member L mem) (hinv : τ.RunInv cod k) :
    (parseFrames (τ.chanBytes cod k src)).1
        = ((τ.stream cod (τ.who k) src k).drop (τ.pc cod k (τ.who k) src)).take
            ((Wire.splitFrames (τ.stream cod (τ.who k) src k) (τ.cur' cod k src)).1 - τ.pc cod k (τ.who k) src)
      ∧ (parseFrames (τ.chanBytes cod k src)).2
        = tailOf (τ.stream cod (τ.who k) src k) (τ.cur' cod k src) := by
  rw [τ.chanBytes_eq cod k src hinv]
  unfold pc tailOf
  have hb : τ.cur cod k (τ.who k) src ≤ (Wire.frameAll (τ.stream cod (τ.who k) src k)).size := hinv.cur_le _ _
  have hb' : τ.cur' cod k src ≤ (Wire.frameAll (τ.stream cod (τ.who k) src k)).size := τ.cur'_le cod k src hb
  have hbb' := τ.cur_le_cur' cod k src
  have hspec := Wire.splitFrames_spec _ _ hb
  have hadd := Wire.splitFrames_add (τ.stream cod (τ.who k) src k) (τ.cur cod k (τ.who k) src)
    (τ.cur' cod k src - τ.cur cod k (τ.who k) src)
  rw [show τ.cur cod k (τ.who k) src + (τ.cur' cod k src - τ.cur cod k (τ.who k) src) = τ.cur' cod k src by omega] at hadd
  generalize τ.stream cod (τ.who k) src k = xs at *
  generalize τ.cur' cod k src = c at *
  generalize τ.cur cod k (τ.who k) src = b at *
  have hsz : (Wire.frameAll xs).size = (Wire.frameAll (xs.take (Wire.splitFrames xs b).1)).size
      + (Wire.frameAll (xs.drop (Wire.splitFrames xs b).1)).size := by
    rw [← ByteArray.size_append, ← Wire.frameAll_append, List.take_append_drop]
  have hbud : (Wire.splitFrames xs b).2 + (c - b) ≤ (Wire.frameAll (xs.drop (Wire.splitFrames xs b).1)).size := by
    omega
  unfold parseFrames Wire.frames
  rw [Wire.framesFrom_prefix_frameAll _ _ hbud, hadd]
  simp only [Array.empty_append, List.toList_toArray, List.filterMap_map, Function.comp_def, Except.toOption,
    List.filterMap_some, Nat.add_sub_cancel_left, List.drop_drop, and_true, true_and]
  try exact ⟨rfl, rfl⟩
  try rfl

/-- `RunInv` at step `0`: cursors at `0`, tails empty. -/
theorem runInv_zero (cod : Dag.Codecs d) (τ : PassTrace d) : τ.RunInv cod 0 :=
  { cur_le := fun _ _ => Nat.zero_le _
    buf_eq := fun m src => by
      show ByteArray.empty = tailOf (streamOf τ.who (τ.sent cod) m src 0) 0
      unfold tailOf streamOf activationsOf
      simp [Wire.splitFrames, Wire.frameAll] }

/-- A tail within the framed prefix does not see messages appended after it. -/
theorem tailOf_append_of_le (xs ys : List WireMsg) (b : Nat) (hb : b ≤ (Wire.frameAll xs).size) :
    tailOf (xs ++ ys) b = tailOf xs b := Wire.tail_append_of_le xs ys b hb

/-- `RunInv` is preserved by a step: the running process's cursor advances within the stream
(`cur'_le`) and its tail is `parse_eq`'s; the others are unchanged and the stream only grows. -/
theorem runInv_succ (cod : Dag.Codecs d) (τ : PassTrace d) (k : Nat) (h : τ.RunInv cod k) : τ.RunInv cod (k + 1) := by
  refine ⟨fun m src => ?_, fun m src => ?_⟩
  · by_cases hm : τ.who k = m
    · subst hm
      show (τ.run cod (k + 1)).cur (τ.who k) src ≤ _
      rw [cur_eq_step, stream_succ]
      exact (τ.cur'_le cod k src (h.cur_le _ _)).trans (size_frameMsgs_le_append _ _)
    · rw [cur_succ_other cod τ k m src hm, stream_succ]
      exact (h.cur_le m src).trans (size_frameMsgs_le_append _ _)
  · by_cases hm : τ.who k = m
    · subst hm
      rw [buf_succ_self, stream_succ, show τ.cur cod (k + 1) (τ.who k) src = τ.cur' cod k src from cur_eq_step cod τ k src,
        tailOf_append_of_le _ _ _ (τ.cur'_le cod k src (h.cur_le _ _))]
      split
      · exact (τ.parse_eq cod k src h).2
      · rename_i hs
        rw [h.buf_eq, cur'_of_not_mem cod τ k src hs]
    · rw [buf_succ_other cod τ k m src hm, cur_succ_other cod τ k m src hm, stream_succ,
        tailOf_append_of_le _ _ _ (h.cur_le m src)]
      exact h.buf_eq m src

/-- `RunInv` holds at every step. -/
theorem runInv (cod : Dag.Codecs d) (τ : PassTrace d) : ∀ k, τ.RunInv cod k
  | 0 => τ.runInv_zero cod
  | k + 1 => τ.runInv_succ cod k (τ.runInv cod k)

/-! ### The parsed counts: idle-stable, within the stream, monotone, before this step's sends -/

/-- The parsed count is within the stream. -/
theorem pc_le (cod : Dag.Codecs d) (τ : PassTrace d) (k : Nat) (m src : Member L mem) :
    τ.pc cod k m src ≤ (τ.stream cod m src k).length := Wire.splitFrames_fst_le_length _ _

/-- Another process's parsed count does not change at step `k`. -/
theorem pc_succ_other (cod : Dag.Codecs d) (τ : PassTrace d) (k : Nat) (m src : Member L mem) (hm : τ.who k ≠ m) :
    τ.pc cod (k + 1) m src = τ.pc cod k m src := by
  unfold pc
  rw [cur_succ_other cod τ k m src hm, stream_succ, Wire.splitFrames_append_of_le _ _ _ ((τ.runInv cod k).cur_le m src)]

/-- The running process's parsed count after step `k`, against the stream before its own sends:
the frames that fit in `cur'`. -/
theorem pc_succ_self (cod : Dag.Codecs d) (τ : PassTrace d) (k : Nat) (src : Member L mem) :
    τ.pc cod (k + 1) (τ.who k) src = (Wire.splitFrames (τ.stream cod (τ.who k) src k) (τ.cur' cod k src)).1 := by
  unfold pc
  rw [show τ.cur cod (k + 1) (τ.who k) src = τ.cur' cod k src from cur_eq_step cod τ k src, stream_succ,
    Wire.splitFrames_append_of_le _ _ _ (τ.cur'_le cod k src ((τ.runInv cod k).cur_le _ _))]

/-- The parsed count never decreases. -/
theorem pc_mono (cod : Dag.Codecs d) (τ : PassTrace d) (k : Nat) (m src : Member L mem) :
    τ.pc cod k m src ≤ τ.pc cod (k + 1) m src := by
  by_cases hm : τ.who k = m
  · subst hm
    rw [pc_succ_self]
    exact Wire.splitFrames_fst_mono _ (τ.cur_le_cur' cod k src)
  · rw [pc_succ_other cod τ k m src hm]

/-- The running process's parsed count after step `k` is within the stream before its own
sends. -/
theorem pc_succ_self_le (cod : Dag.Codecs d) (τ : PassTrace d) (k : Nat) (src : Member L mem) :
    τ.pc cod (k + 1) (τ.who k) src ≤ (τ.stream cod (τ.who k) src k).length := by
  rw [pc_succ_self]
  exact Wire.splitFrames_fst_le_length _ _

/-- A connection not read keeps its parsed count. -/
theorem pc_succ_self_of_not_mem (cod : Dag.Codecs d) (τ : PassTrace d) (k : Nat) (src : Member L mem)
    (h : src ∉ srcsOf (τ.reads k)) : τ.pc cod (k + 1) (τ.who k) src = τ.pc cod k (τ.who k) src := by
  rw [pc_succ_self, cur'_of_not_mem cod τ k src h]
  rfl

/-- **The batches activation `k` parses**: per connection read, the next messages of its stream
from the old parsed count up to the new one (`parse_eq`, `pc_succ_self`). -/
theorem batches_eq (cod : Dag.Codecs d) (τ : PassTrace d) (k : Nat) :
    τ.batches cod k
      = (srcsOf (τ.reads k)).map fun src =>
          ⟨src, ((τ.stream cod (τ.who k) src k).drop (τ.pc cod k (τ.who k) src)).take
            (τ.pc cod (k + 1) (τ.who k) src - τ.pc cod k (τ.who k) src)⟩ := by
  unfold batches
  apply List.map_congr_left
  intro src _
  rw [(τ.parse_eq cod k src (τ.runInv cod k)).1, pc_succ_self]

end PassTrace

/-! ## The induced schedule and the queue invariant

Both are read off the trace: `who`, what each activation put on each connection (`sent`), the
tick flags (`tick`), and the parsed counts (`pc k m src`: how many of `src`'s messages to `m`
the receiver has parsed before step `k`). At a network node, the cursor of receiver `i` from
sender `j` at step `t` is the number of that node's messages among the parsed prefix at
`t + 1`, so the activation at `t` is included if it is `i`'s; at a timer node the script is the
readings at the member's `n`-th tick. The invariant `TraceQueues` says what the lock-step cells
of member `m` hold at step `k` relative to those counts. The facts about `pc` the proofs need
are the four in `PcOk`; the trace's counts satisfy them (`PassTrace.pcOk`). -/

section Induced

variable (who : Nat → Member L mem) (sent : Nat → Member L mem → List WireMsg) (pc : Nat → Member L mem → Member L mem → Nat)
  (tick : Nat → Bool)

/-- What the induction needs of the parsed counts: stable at a process that is not running,
within the stream, monotone, and after the running process's step within the stream before its
own sends. -/
structure PcOk : Prop where
  idle : ∀ k m src, who k ≠ m → pc (k + 1) m src = pc k m src
  le : ∀ k m src, pc k m src ≤ (streamOf who sent m src k).length
  mono : ∀ k m src, pc k m src ≤ pc (k + 1) m src
  self_le : ∀ k src, pc (k + 1) (who k) src ≤ (streamOf who sent (who k) src k).length

/-- The messages of a connection `m` has not yet parsed before step `k`, as one node sees them:
`f` selects and decodes the node's messages from that sender. -/
def unconsumed {α : Type} (f : WireMsg → Option α) (m src : Member L mem) (k : Nat) : List α :=
  ((streamOf who sent m src k).drop (pc k m src)).filterMap f

/-- The count of the node's messages `m` has parsed from `src` before step `k`. -/
def ccount {α : Type} (f : WireMsg → Option α) (m src : Member L mem) (k : Nat) : Nat :=
  (((streamOf who sent m src k).take (pc k m src)).filterMap f).length

/-- The node's messages among those activation `k` parses from `src`. -/
def newMsgs {α : Type} (f : WireMsg → Option α) (src : Member L mem) (k : Nat) : List α :=
  (((streamOf who sent (who k) src k).drop (pc k (who k) src)).take (pc (k + 1) (who k) src - pc k (who k) src)).filterMap f

/-- A `broadcast_closed` node's messages from sender `j`, decoded. -/
def bcSel {α : Type} (cod : Codec α) (base : Nat) (j : Nat) (msg : WireMsg) : Option α :=
  if msg.1 = base ∧ msg.2.1 = j then (cod.dec msg.2.2).toOption else none

/-- A `demux` node's messages from sender `j` addressed to receiver `i`, decoded. -/
def dxSel {α : Type} {p : L} (cod : Codec (Nat × α)) (base : Nat) (addr : Fin (mem p) → Nat) (i : Fin (mem p)) (j : Nat)
    (msg : WireMsg) : Option α :=
  (bcSel cod base j msg).bind fun dx => if dx.1 = addr i then some dx.2 else none

/-- `decodeAt` as a `filterMap` by `bcSel`. -/
theorem decodeAt_eq_filterMap {α : Type} (cod : Codec α) (msgs : List WireMsg) (base j : Nat) :
    (decodeAt cod msgs base j).toList = msgs.filterMap (bcSel cod base j) := by
  rw [decodeAt_toList]; rfl

/-- `decodeAt` filtered to a receiver's address as a `filterMap` by `dxSel`. -/
theorem decodeAt_filterMap_eq {α : Type} {p : L} (cod : Codec (Nat × α)) (msgs : List WireMsg) (base : Nat)
    (addr : Fin (mem p) → Nat) (i : Fin (mem p)) (j : Nat) :
    (filterMapB (fun dx => if dx.1 = addr i then some dx.2 else none) (decodeAt cod msgs base j)).toList
      = msgs.filterMap (dxSel cod base addr i j) := by
  rw [toList_filterMapB, decodeAt_eq_filterMap, List.filterMap_filterMap]
  rfl

mutual

/-- The schedule a trace induces at a node. A network node's cursor for receiver `i` from sender
`j` at step `t` is `ccount` at `t + 1`; a timer's script entry `n` is the reading at the member's
`n`-th tick (`nthTick`), `0` if it has not happened. Noncomputable through `nthTick`. -/
noncomputable def Node.traceSched (readings : (k : Nat) → TickReadings (who k)) :
    {Γ : Ctx L mem} → {κs : List (Kind L mem)} → (n : Node Γ κs) → Node.Codecs n → Nat → Node.Sched n
  | _, _, .broadcast_closed (c := c) (p := p) _ _, cod, base => fun i j t =>
    ccount who sent pc (bcSel cod base j.val) ⟨p, i⟩ ⟨c, j⟩ (t + 1)
  | _, _, .demux (c := c) (p := p) _ _ addr, cod, base => fun i j t =>
    ccount who sent pc (dxSel cod base addr i j.val) ⟨p, i⟩ ⟨c, j⟩ (t + 1)
  | _, _, .source_interval (ℓ := ℓ) delay period, _, _ => fun i n =>
    match nthTick who tick ⟨ℓ, i⟩ n with
    | some k => (readings k.1).fires ℓ ⟨i, k.2.symm⟩ (delay i) period
    | none => 0
  | _, _, .time (ℓ := ℓ), _, _ => fun i n =>
    match nthTick who tick ⟨ℓ, i⟩ n with
    | some k => (readings k.1).clock ℓ ⟨i, k.2.symm⟩
    | none => 0
  | _, _, .call N _, cod, base => Dag.traceSched readings N cod base
  | _, _, .fixN _ body, cod, base => Dag.traceSched readings body cod base
  | _, _, .map .., _, _ => ()
  | _, _, .filterMap .., _, _ => ()
  | _, _, .values .., _, _ => ()
  | _, _, .into_keyed .., _, _ => ()
  | _, _, .kassume_ordering .., _, _ => ()
  | _, _, .kfold_early_stop .., _, _ => ()
  | _, _, .kget_max_key .., _, _ => ()
  | _, _, .weaken_retries .., _, _ => ()
  | _, _, .union .., _, _ => ()
  | _, _, .assume_ordering .., _, _ => ()
  | _, _, .fold .., _, _ => ()
  | _, _, .fold_monotone .., _, _ => ()
  | _, _, .snapshot .., _, _ => ()
  | _, _, .batch .., _, _ => ()
  | _, _, .mapTick .., _, _ => ()
  | _, _, .firstTick .., _, _ => ()
  | _, _, .zipTick .., _, _ => ()
  | _, _, .defer_tick .., _, _ => ()
  | _, _, .allTicks .., _, _ => ()
  | _, _, .flattenOrdered .., _, _ => ()
  | _, _, .flattenUnordered .., _, _ => ()
  | _, _, .tinto_keyed .., _, _ => ()
  | _, _, .tassume_ordering .., _, _ => ()
  | _, _, .kentries .., _, _ => ()
  | _, _, .kmap .., _, _ => ()
  | _, _, .kallTicks .., _, _ => ()
  | _, _, .kfold .., _, _ => ()
  | _, _, .ksnapshot .., _, _ => ()
  | _, _, .tfold .., _, _ => ()
  | _, _, .latest .., _, _ => ()
  | _, _, .latest_monotone .., _, _ => ()
  | _, _, .snapshot_bounded .., _, _ => ()
  | _, _, .reduce_watermark .., _, _ => ()
  | _, _, .tick_scan .., _, _ => ()

/-- `Node.traceSched` along the `letN` chain, threading the wire base through
`Node.wireReceivers`. -/
noncomputable def Dag.traceSched (readings : (k : Nat) → TickReadings (who k)) :
    {Γ : Ctx L mem} → {outs : List (Kind L mem)} → (d : Dag Γ outs) → Dag.Codecs d → Nat → Dag.Sched d
  | _, _, .ret _, _, _ => ()
  | _, _, .letN n rest, cod, base =>
    (Node.traceSched readings n cod.1 base, Dag.traceSched readings rest cod.2 (Node.wireReceivers n base).2)

end

mutual

/-- **The queue invariant** at step `k` for member `m`'s cells: each receiving end holds the
connection's unparsed messages (`unconsumed`) and the parsed count (`ccount`); each timer cell
holds `m`'s tick count. -/
def Node.TraceQueues (k : Nat) (m : Member L mem) :
    {Γ : Ctx L mem} → {κs : List (Kind L mem)} → (n : Node Γ κs) → Node.Codecs n → Nat → Node.LocState m n → Prop
  | _, _, .broadcast_closed (c := c) _ _, cod, base, s => ∀ i j,
    s i j = ⟨(unconsumed who sent pc (bcSel cod base j.val) m ⟨c, j⟩ k).toArray,
      ccount who sent pc (bcSel cod base j.val) m ⟨c, j⟩ k⟩
  | _, _, .demux (c := c) _ _ addr, cod, base, s => ∀ i j,
    s i j = ⟨(unconsumed who sent pc (dxSel cod base addr i.1 j.val) m ⟨c, j⟩ k).toArray,
      ccount who sent pc (dxSel cod base addr i.1 j.val) m ⟨c, j⟩ k⟩
  | _, _, .source_interval .., _, _, s => ∀ i, s i = (ticksOf who tick m k).length
  | _, _, .time .., _, _, s => ∀ i, s i = (ticksOf who tick m k).length
  | _, _, .call N _, cod, base, s => Dag.TraceQueues k m N cod base s
  | _, _, .fixN _ body, cod, base, s => Dag.TraceQueues k m body cod base s
  | _, _, .map .., _, _, _ => True
  | _, _, .filterMap .., _, _, _ => True
  | _, _, .values .., _, _, _ => True
  | _, _, .into_keyed .., _, _, _ => True
  | _, _, .kassume_ordering .., _, _, _ => True
  | _, _, .kfold_early_stop .., _, _, _ => True
  | _, _, .kget_max_key .., _, _, _ => True
  | _, _, .weaken_retries .., _, _, _ => True
  | _, _, .union .., _, _, _ => True
  | _, _, .assume_ordering .., _, _, _ => True
  | _, _, .fold .., _, _, _ => True
  | _, _, .fold_monotone .., _, _, _ => True
  | _, _, .snapshot .., _, _, _ => True
  | _, _, .batch .., _, _, _ => True
  | _, _, .mapTick .., _, _, _ => True
  | _, _, .firstTick .., _, _, _ => True
  | _, _, .zipTick .., _, _, _ => True
  | _, _, .defer_tick .., _, _, _ => True
  | _, _, .allTicks .., _, _, _ => True
  | _, _, .flattenOrdered .., _, _, _ => True
  | _, _, .flattenUnordered .., _, _, _ => True
  | _, _, .tinto_keyed .., _, _, _ => True
  | _, _, .tassume_ordering .., _, _, _ => True
  | _, _, .kentries .., _, _, _ => True
  | _, _, .kmap .., _, _, _ => True
  | _, _, .kallTicks .., _, _, _ => True
  | _, _, .kfold .., _, _, _ => True
  | _, _, .ksnapshot .., _, _, _ => True
  | _, _, .tfold .., _, _, _ => True
  | _, _, .latest .., _, _, _ => True
  | _, _, .latest_monotone .., _, _, _ => True
  | _, _, .snapshot_bounded .., _, _, _ => True
  | _, _, .reduce_watermark .., _, _, _ => True
  | _, _, .tick_scan .., _, _, _ => True

/-- `Node.TraceQueues` along the `letN` chain. -/
def Dag.TraceQueues (k : Nat) (m : Member L mem) :
    {Γ : Ctx L mem} → {outs : List (Kind L mem)} → (d : Dag Γ outs) → Dag.Codecs d → Nat → Dag.LocState m d → Prop
  | _, _, .ret _, _, _, _ => True
  | _, _, .letN n rest, cod, base, s =>
    Node.TraceQueues k m n cod.1 base s.1 ∧ Dag.TraceQueues k m rest cod.2 (Node.wireReceivers n base).2 s.2

end

/-! ### Facts about one connection -/


/-- Selecting one connection `x` from a batch list over distinct senders yields `g` of `x`'s
batch; a sender not in the list has an empty batch (`hx`), and `g [] = []`. -/
theorem flatMap_batches {α : Type} (srcs : List (Member L mem)) (hnd : srcs.Nodup) (N : Member L mem → List WireMsg)
    (g : List WireMsg → List α) (hg : g [] = []) (x : Member L mem) (hx : x ∉ srcs → N x = []) :
    ((srcs.map fun s => (⟨s, N s⟩ : (src : Member L mem) × List WireMsg)).flatMap fun b => if x = b.1 then g b.2 else [])
      = g (N x) := by
  induction srcs with
  | nil => rw [List.map_nil, List.flatMap_nil, hx List.not_mem_nil, hg]
  | cons s srcs ih =>
    rw [List.nodup_cons] at hnd
    rw [List.map_cons, List.flatMap_cons]
    by_cases hxs : x = s
    · subst hxs
      simp only [if_true]
      have : ((srcs.map fun s => (⟨s, N s⟩ : (src : Member L mem) × List WireMsg)).flatMap
          fun b => if x = b.1 then g b.2 else []) = [] := by
        rw [List.flatMap_eq_nil_iff]
        intro b hb
        obtain ⟨s', hs', rfl⟩ := List.mem_map.mp hb
        rw [if_neg]
        intro h; exact hnd.1 (h ▸ hs')
      rw [this, List.append_nil]
    · simp only [hxs, if_false, List.nil_append]
      exact ih hnd.2 (fun h => hx (fun h' => (List.mem_cons.mp h').elim hxs h))


/-- `flatMap_batches` for `appendFold` over arrays. -/
theorem appendFold_batches {α : Type} (srcs : List (Member L mem)) (hnd : srcs.Nodup) (N : Member L mem → List WireMsg)
    (g : List WireMsg → Array α) (hg : g [] = #[]) (x : Member L mem) (hx : x ∉ srcs → N x = []) :
    appendFold (fun b : (src : Member L mem) × List WireMsg => if x = b.1 then g b.2 else #[])
        (srcs.map fun s => (⟨s, N s⟩ : (src : Member L mem) × List WireMsg))
      = g (N x) := by
  apply Array.toList_inj.mp
  rw [appendFold_toList]
  have := flatMap_batches srcs hnd N (fun l => (g l).toList) (by rw [hg]) x hx
  rw [← this]
  apply List.flatMap_congr
  intro b _
  rw [apply_ite Array.toList, Array.toList_empty]

/-- `cumMax` of a monotone sequence is the sequence. -/
theorem cumMax_eq_of_mono (c : Nat → Nat) (h : ∀ t, c t ≤ c (t + 1)) : ∀ t, cumMax c t = c t
  | 0 => rfl
  | t + 1 => by
    show max (cumMax c t) (c (t + 1)) = c (t + 1)
    rw [cumMax_eq_of_mono c h t]
    exact Nat.max_eq_right (h t)

variable (hpc : PcOk who sent pc)
include hpc

/-- The parsed count of a node's messages never decreases (`hpc.mono`). -/
theorem ccount_mono {α : Type} (f : WireMsg → Option α) (m src : Member L mem) (k : Nat) :
    ccount who sent pc f m src k ≤ ccount who sent pc f m src (k + 1) := by
  unfold ccount
  rw [streamOf_succ, show pc (k + 1) m src = pc k m src + (pc (k + 1) m src - pc k m src) by
    have := hpc.mono k m src; omega,
    List.take_add, List.take_append_of_le_length (hpc.le k m src), List.filterMap_append, List.length_append]
  exact Nat.le_add_right _ _

/-- The induced cursor at `k` is `ccount` at `k + 1`: `cumMax` is transparent on the monotone
counts. -/
theorem cursor_eq {α : Type} (f : WireMsg → Option α) (m src : Member L mem) (k : Nat) :
    cumMax (fun t => ccount who sent pc f m src (t + 1)) k = ccount who sent pc f m src (k + 1) :=
  cumMax_eq_of_mono _ (fun t => ccount_mono who sent pc hpc f m src (t + 1)) k

/-- A receiving end's cell at a step where the receiver `m` is idle and the sender `src` runs:
enqueueing the sender's decoded messages for `m` (`ha`) re-establishes the invariant at
`k + 1`. -/
theorem chan_idle_sender {α : Type} (f : WireMsg → Option α) (m src : Member L mem) (k : Nat) (hm : who k ≠ m)
    (hsrc : who k = src) (a : Array α) (ha : a.toList = (sent k m).filterMap f) (cell : ChanCell α)
    (hcell : cell = ⟨(unconsumed who sent pc f m src k).toArray, ccount who sent pc f m src k⟩) :
    cell.enqueue a = ⟨(unconsumed who sent pc f m src (k + 1)).toArray, ccount who sent pc f m src (k + 1)⟩ := by
  rw [hcell, ChanCell.enqueue_eq]
  simp only [List.toList_toArray]
  rw [ha]
  unfold unconsumed ccount
  rw [hpc.idle k m src hm, streamOf_succ, if_pos hsrc, List.drop_append_of_le_length (hpc.le k m src),
    List.take_append_of_le_length (hpc.le k m src), List.filterMap_append]

/-- A receiving end's cell at a step where neither the receiver nor the sender runs: nothing
arrives and the invariant at `k + 1` is the cell unchanged. -/
theorem chan_idle_other {α : Type} (f : WireMsg → Option α) (m src : Member L mem) (k : Nat) (hm : who k ≠ m)
    (hsrc : who k ≠ src) (cell : ChanCell α)
    (hcell : cell = ⟨(unconsumed who sent pc f m src k).toArray, ccount who sent pc f m src k⟩) :
    cell = ⟨(unconsumed who sent pc f m src (k + 1)).toArray, ccount who sent pc f m src (k + 1)⟩ := by
  rw [hcell]
  unfold unconsumed ccount
  rw [hpc.idle k m src hm, streamOf_succ, if_neg hsrc, List.append_nil]

/-- The unparsed messages before the receiver's own activation split into what it parses now
(`newMsgs`) and what remains past the new parsed count. -/
theorem unconsumed_split {α : Type} (f : WireMsg → Option α) (src : Member L mem) (k : Nat) :
    unconsumed who sent pc f (who k) src k
      = newMsgs who sent pc f src k ++ ((streamOf who sent (who k) src k).drop (pc (k + 1) (who k) src)).filterMap f := by
  unfold unconsumed newMsgs
  have h1 : (streamOf who sent (who k) src k).drop (pc (k + 1) (who k) src)
      = ((streamOf who sent (who k) src k).drop (pc k (who k) src)).drop (pc (k + 1) (who k) src - pc k (who k) src) := by
    rw [List.drop_drop]
    congr 1
    have := hpc.mono k (who k) src
    omega
  rw [h1, ← List.filterMap_append, List.take_append_drop]

/-- The running process's parsed count of a node's messages grows by the newly parsed ones;
`hpc.self_le` keeps the count within the stream before its own sends. -/
theorem ccount_active {α : Type} (f : WireMsg → Option α) (src : Member L mem) (k : Nat) :
    ccount who sent pc f (who k) src (k + 1) = ccount who sent pc f (who k) src k + (newMsgs who sent pc f src k).length := by
  unfold ccount newMsgs
  rw [streamOf_succ, List.take_append_of_le_length (hpc.self_le k src),
    show pc (k + 1) (who k) src = pc k (who k) src + (pc (k + 1) (who k) src - pc k (who k) src) by
      have := hpc.mono k (who k) src; omega,
    List.take_add, List.filterMap_append, List.length_append]
  simp only [Nat.add_sub_cancel_left]

/-- A receiving end's cell through the receiver's own activation `k`, released to the induced
cursor: the newly parsed messages are delivered and the remainder stays pending. -/
theorem chan_active_step {α : Type} (f : WireMsg → Option α) (src : Member L mem) (k : Nat) (cell : ChanCell α)
    (hcell : cell = ⟨(unconsumed who sent pc f (who k) src k).toArray, ccount who sent pc f (who k) src k⟩) :
    (cell.step (ccount who sent pc f (who k) src (k + 1)) #[]).1
      = ⟨(((streamOf who sent (who k) src k).drop (pc (k + 1) (who k) src)).filterMap f).toArray,
         ccount who sent pc f (who k) src (k + 1)⟩ := by
  rw [hcell, unconsumed_split who sent pc hpc f src k, ccount_active who sent pc hpc f src k, ChanCell.step_eq]
  simp only [List.toList_toArray, Nat.add_sub_cancel_left, List.take_left, List.drop_left, List.append_nil,
    Array.toList_empty]

/-- `chan_active_step` followed by enqueueing the process's own sends, when it is also the
sender (the loopback connection): the invariant at `k + 1`. -/
theorem chan_active_sender {α : Type} (f : WireMsg → Option α) (src : Member L mem) (k : Nat) (hsrc : who k = src)
    (a : Array α) (ha : a.toList = (sent k (who k)).filterMap f) (cell : ChanCell α)
    (hcell : cell = ⟨(unconsumed who sent pc f (who k) src k).toArray, ccount who sent pc f (who k) src k⟩) :
    (cell.step (ccount who sent pc f (who k) src (k + 1)) #[]).1.enqueue a
      = ⟨(unconsumed who sent pc f (who k) src (k + 1)).toArray, ccount who sent pc f (who k) src (k + 1)⟩ := by
  rw [chan_active_step who sent pc hpc f src k cell hcell, ChanCell.enqueue_eq]
  simp only [List.toList_toArray]
  rw [ha]
  unfold unconsumed
  rw [streamOf_succ, if_pos hsrc, List.drop_append_of_le_length (hpc.self_le k src), List.filterMap_append]

/-- `chan_active_step` when the running process is not the sender: the invariant at `k + 1`
with nothing enqueued. -/
theorem chan_active_other {α : Type} (f : WireMsg → Option α) (src : Member L mem) (k : Nat) (hsrc : who k ≠ src)
    (cell : ChanCell α)
    (hcell : cell = ⟨(unconsumed who sent pc f (who k) src k).toArray, ccount who sent pc f (who k) src k⟩) :
    (cell.step (ccount who sent pc f (who k) src (k + 1)) #[]).1
      = ⟨(unconsumed who sent pc f (who k) src (k + 1)).toArray, ccount who sent pc f (who k) src (k + 1)⟩ := by
  rw [chan_active_step who sent pc hpc f src k cell hcell]
  unfold unconsumed
  rw [streamOf_succ, if_neg hsrc, List.append_nil]

/-- What the induced cursor delivers from a cell satisfying the invariant is the newly parsed
messages. -/
theorem chan_arrivals {α : Type} (f : WireMsg → Option α) (src : Member L mem) (k : Nat) (cell : ChanCell α)
    (hcell : cell = ⟨(unconsumed who sent pc f (who k) src k).toArray, ccount who sent pc f (who k) src k⟩) :
    cell.pending.extract 0 (ccount who sent pc f (who k) src (k + 1) - cell.delivered)
      = (newMsgs who sent pc f src k).toArray := by
  rw [hcell, unconsumed_split who sent pc hpc f src k, ccount_active who sent pc hpc f src k]
  simp only [List.extract_zero_toArray, Nat.add_sub_cancel_left, List.take_left]

/-! ### The invariant along the lock-step run -/

omit hpc in
mutual

/-- The initial cells satisfy the invariant at step `0`: empty queues, zero counts, no ticks. -/
theorem Node.traceQueues_init (m : Member L mem) : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → (n : Node Γ κs) →
    (cod : Node.Codecs n) → (base : Nat) →
    Node.TraceQueues who sent pc tick 0 m n cod base (Node.restrict m n (Node.init n))
  | _, _, .broadcast_closed .., _, _ => fun _ _ => by
    show ChanCell.init = _
    unfold unconsumed ccount streamOf activationsOf
    simp [ChanCell.init]
  | _, _, .demux .., _, _ => fun _ _ => by
    show ChanCell.init = _
    unfold unconsumed ccount streamOf activationsOf
    simp [ChanCell.init]
  | _, _, .source_interval .., _, _ => fun _ => rfl
  | _, _, .time .., _, _ => fun _ => rfl
  | _, _, .call N _, cod, base => Dag.traceQueues_init m N cod base
  | _, _, .fixN _ body, cod, base => Dag.traceQueues_init m body cod base
  | _, _, .map .., _, _ => trivial
  | _, _, .filterMap .., _, _ => trivial
  | _, _, .values .., _, _ => trivial
  | _, _, .into_keyed .., _, _ => trivial
  | _, _, .kassume_ordering .., _, _ => trivial
  | _, _, .kfold_early_stop .., _, _ => trivial
  | _, _, .kget_max_key .., _, _ => trivial
  | _, _, .weaken_retries .., _, _ => trivial
  | _, _, .union .., _, _ => trivial
  | _, _, .assume_ordering .., _, _ => trivial
  | _, _, .fold .., _, _ => trivial
  | _, _, .fold_monotone .., _, _ => trivial
  | _, _, .snapshot .., _, _ => trivial
  | _, _, .batch .., _, _ => trivial
  | _, _, .mapTick .., _, _ => trivial
  | _, _, .firstTick .., _, _ => trivial
  | _, _, .zipTick .., _, _ => trivial
  | _, _, .defer_tick .., _, _ => trivial
  | _, _, .allTicks .., _, _ => trivial
  | _, _, .flattenOrdered .., _, _ => trivial
  | _, _, .flattenUnordered .., _, _ => trivial
  | _, _, .tinto_keyed .., _, _ => trivial
  | _, _, .tassume_ordering .., _, _ => trivial
  | _, _, .kentries .., _, _ => trivial
  | _, _, .kmap .., _, _ => trivial
  | _, _, .kallTicks .., _, _ => trivial
  | _, _, .kfold .., _, _ => trivial
  | _, _, .ksnapshot .., _, _ => trivial
  | _, _, .tfold .., _, _ => trivial
  | _, _, .latest .., _, _ => trivial
  | _, _, .latest_monotone .., _, _ => trivial
  | _, _, .snapshot_bounded .., _, _ => trivial
  | _, _, .reduce_watermark .., _, _ => trivial
  | _, _, .tick_scan .., _, _ => trivial

/-- `Node.traceQueues_init` along the `letN` chain. -/
theorem Dag.traceQueues_init (m : Member L mem) : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → (d : Dag Γ outs) →
    (cod : Dag.Codecs d) → (base : Nat) →
    Dag.TraceQueues who sent pc tick 0 m d cod base (Dag.restrict m d (Dag.init d))
  | _, _, .ret _, _, _ => trivial
  | _, _, .letN n rest, cod, base =>
    ⟨Node.traceQueues_init m n cod.1 base, Dag.traceQueues_init m rest cod.2 _⟩

end

mutual

/-- **Idle step**: a process `m` that is not running (`hm`) receives the active one's bytes
through `Node.inboxOfWire`, and the invariant moves to `k + 1` (`chan_idle_sender`,
`chan_idle_other`; the tick count is unchanged). -/
theorem Node.traceQueues_idle (k : Nat) (m : Member L mem) (hm : who k ≠ m) :
    {Γ : Ctx L mem} → {κs : List (Kind L mem)} → (n : Node Γ κs) → (cod : Node.Codecs n) → (base : Nat) →
    (s : Node.LocState m n) → Node.TraceQueues who sent pc tick k m n cod base s →
    Node.TraceQueues who sent pc tick (k + 1) m n cod base
      (Node.enqueueInbox m n (Node.inboxOfWire m n cod base [⟨who k, sent k m⟩]).1 s)
  | _, _, .broadcast_closed (c := c) _ _, cod, base, s, h => fun i j => by
    show (s i j).enqueue (appendFold (fun b : (src : Member L mem) × List WireMsg =>
        if (⟨c, j⟩ : Member L mem) = b.1 then decodeAt cod b.2 base j.val else #[]) [⟨who k, sent k m⟩]) = _
    rw [appendFold_singleton]
    show (s i j).enqueue (if (⟨c, j⟩ : Member L mem) = who k then decodeAt cod (sent k m) base j.val else #[]) = _
    by_cases hs : (⟨c, j⟩ : Member L mem) = who k
    · rw [if_pos hs]
      exact chan_idle_sender who sent pc hpc (bcSel cod base j.val) m ⟨c, j⟩ k hm hs.symm _
        (decodeAt_eq_filterMap cod _ base j.val) (s i j) (h i j)
    · rw [if_neg hs, ChanCell.enqueue_nil]
      exact chan_idle_other who sent pc hpc (bcSel cod base j.val) m ⟨c, j⟩ k hm (Ne.symm hs) (s i j) (h i j)
  | _, _, .demux (c := c) _ _ addr, cod, base, s, h => fun i j => by
    show (s i j).enqueue (appendFold (fun b : (src : Member L mem) × List WireMsg =>
        if (⟨c, j⟩ : Member L mem) = b.1 then
          filterMapB (fun dx => if dx.1 = addr i.1 then some dx.2 else none) (decodeAt cod b.2 base j.val)
        else #[]) [⟨who k, sent k m⟩]) = _
    rw [appendFold_singleton]
    show (s i j).enqueue (if (⟨c, j⟩ : Member L mem) = who k then
        filterMapB (fun dx => if dx.1 = addr i.1 then some dx.2 else none) (decodeAt cod (sent k m) base j.val)
      else #[]) = _
    by_cases hs : (⟨c, j⟩ : Member L mem) = who k
    · rw [if_pos hs]
      exact chan_idle_sender who sent pc hpc (dxSel cod base addr i.1 j.val) m ⟨c, j⟩ k hm hs.symm _
        (decodeAt_filterMap_eq _ _ _ _ _ _) (s i j) (h i j)
    · rw [if_neg hs, ChanCell.enqueue_nil]
      exact chan_idle_other who sent pc hpc (dxSel cod base addr i.1 j.val) m ⟨c, j⟩ k hm (Ne.symm hs) (s i j) (h i j)
  | _, _, .source_interval .., _, _, s, h => fun i => by
    show s i = _
    rw [h i, ticksOf_succ, if_neg (fun hh => hm hh.1), List.append_nil]
  | _, _, .time .., _, _, s, h => fun i => by
    show s i = _
    rw [h i, ticksOf_succ, if_neg (fun hh => hm hh.1), List.append_nil]
  | _, _, .call N _, cod, base, s, h => Dag.traceQueues_idle k m hm N cod base s h
  | _, _, .fixN _ body, cod, base, s, h => Dag.traceQueues_idle k m hm body cod base s h
  | _, _, .map .., _, _, _, _ => trivial
  | _, _, .filterMap .., _, _, _, _ => trivial
  | _, _, .values .., _, _, _, _ => trivial
  | _, _, .into_keyed .., _, _, _, _ => trivial
  | _, _, .kassume_ordering .., _, _, _, _ => trivial
  | _, _, .kfold_early_stop .., _, _, _, _ => trivial
  | _, _, .kget_max_key .., _, _, _, _ => trivial
  | _, _, .weaken_retries .., _, _, _, _ => trivial
  | _, _, .union .., _, _, _, _ => trivial
  | _, _, .assume_ordering .., _, _, _, _ => trivial
  | _, _, .fold .., _, _, _, _ => trivial
  | _, _, .fold_monotone .., _, _, _, _ => trivial
  | _, _, .snapshot .., _, _, _, _ => trivial
  | _, _, .batch .., _, _, _, _ => trivial
  | _, _, .mapTick .., _, _, _, _ => trivial
  | _, _, .firstTick .., _, _, _, _ => trivial
  | _, _, .zipTick .., _, _, _, _ => trivial
  | _, _, .defer_tick .., _, _, _, _ => trivial
  | _, _, .allTicks .., _, _, _, _ => trivial
  | _, _, .flattenOrdered .., _, _, _, _ => trivial
  | _, _, .flattenUnordered .., _, _, _, _ => trivial
  | _, _, .tinto_keyed .., _, _, _, _ => trivial
  | _, _, .tassume_ordering .., _, _, _, _ => trivial
  | _, _, .kentries .., _, _, _, _ => trivial
  | _, _, .kmap .., _, _, _, _ => trivial
  | _, _, .kallTicks .., _, _, _, _ => trivial
  | _, _, .kfold .., _, _, _, _ => trivial
  | _, _, .ksnapshot .., _, _, _, _ => trivial
  | _, _, .tfold .., _, _, _, _ => trivial
  | _, _, .latest .., _, _, _, _ => trivial
  | _, _, .latest_monotone .., _, _, _, _ => trivial
  | _, _, .snapshot_bounded .., _, _, _, _ => trivial
  | _, _, .reduce_watermark .., _, _, _, _ => trivial
  | _, _, .tick_scan .., _, _, _, _ => trivial

/-- `Node.traceQueues_idle` along the `letN` chain. -/
theorem Dag.traceQueues_idle (k : Nat) (m : Member L mem) (hm : who k ≠ m) :
    {Γ : Ctx L mem} → {outs : List (Kind L mem)} → (d : Dag Γ outs) → (cod : Dag.Codecs d) → (base : Nat) →
    (s : Dag.LocState m d) → Dag.TraceQueues who sent pc tick k m d cod base s →
    Dag.TraceQueues who sent pc tick (k + 1) m d cod base
      (Dag.enqueueInbox m d (Dag.inboxOfWire m d cod base [⟨who k, sent k m⟩]).1 s)
  | _, _, .ret _, _, _, _, _ => trivial
  | _, _, .letN n rest, cod, base, s, h => by
    refine ⟨Node.traceQueues_idle k m hm n cod.1 base s.1 h.1, ?_⟩
    show Dag.TraceQueues who sent pc tick (k + 1) m rest cod.2 _
      (Dag.enqueueInbox m rest (Dag.inboxOfWire m rest cod.2 (Node.inboxOfWire m n cod.1 base _).2 _).1 s.2)
    rw [Node.inboxOfWire_snd_eq]
    exact Dag.traceQueues_idle k m hm rest cod.2 _ s.2 h.2

end

/-- The cursor the induced schedule hands a receiving end of the running process at step `k`: the
parsed count through `k`. -/
theorem traceSched_cursor {α : Type} (f : WireMsg → Option α) {c p : L} (k : Nat) (i : Hosted (who k) p) (j : Fin (mem c)) :
    cumMax (fun t => ccount who sent pc f ⟨p, i.1⟩ ⟨c, j⟩ (t + 1)) k = ccount who sent pc f (who k) ⟨c, j⟩ (k + 1) := by
  rw [cursor_eq who sent pc hpc, i.2]

mutual

/-- **Active step**: the running process's `Node.locStep` under `Node.cursorsAt` on the induced
schedule delivers the newly parsed messages (`chan_active_step`), counts its tick, and then
receives its own sends (`chan_active_sender`); the invariant moves to `k + 1`. -/
theorem Node.traceQueues_active (readings : (k : Nat) → TickReadings (who k)) (k : Nat) :
    {Γ : Ctx L mem} → {κs : List (Kind L mem)} → (n : Node Γ κs) → (cod : Node.Codecs n) → (base : Nat) →
    (Δ : HList (Kind.locDelta (who k)) Γ) → (s : Node.LocState (who k) n) →
    Node.TraceQueues who sent pc tick k (who k) n cod base s →
    Node.TraceQueues who sent pc tick (k + 1) (who k) n cod base
      (Node.enqueueInbox (who k) n (Node.inboxOfWire (who k) n cod base [⟨who k, sent k (who k)⟩]).1
        (Node.locStep (who k) (tick k) n (Node.cursorsAt (who k) n (Node.traceSched who sent pc tick readings n cod base) k s) Δ s).1)
  | _, _, .broadcast_closed (c := c) _ _, cod, base, Δ, s, h => fun i j => by
    show (((s i j).step (cumMax (fun t => ccount who sent pc (bcSel cod base j.val) ⟨_, i.1⟩ ⟨c, j⟩ (t + 1)) k) #[]).1).enqueue
        (appendFold (fun b : (src : Member L mem) × List WireMsg =>
          if (⟨c, j⟩ : Member L mem) = b.1 then decodeAt cod b.2 base j.val else #[]) [⟨who k, sent k (who k)⟩]) = _
    rw [appendFold_singleton, traceSched_cursor who sent pc hpc]
    show (((s i j).step _ #[]).1).enqueue
        (if (⟨c, j⟩ : Member L mem) = who k then decodeAt cod (sent k (who k)) base j.val else #[]) = _
    by_cases hs : (⟨c, j⟩ : Member L mem) = who k
    · rw [if_pos hs]
      exact chan_active_sender who sent pc hpc (bcSel cod base j.val) ⟨c, j⟩ k hs.symm _
        (decodeAt_eq_filterMap cod _ base j.val) (s i j) (h i j)
    · rw [if_neg hs, ChanCell.enqueue_nil]
      exact chan_active_other who sent pc hpc (bcSel cod base j.val) ⟨c, j⟩ k (Ne.symm hs) (s i j) (h i j)
  | _, _, .demux (c := c) _ _ addr, cod, base, Δ, s, h => fun i j => by
    show (((s i j).step (cumMax (fun t => ccount who sent pc (dxSel cod base addr i.1 j.val) ⟨_, i.1⟩ ⟨c, j⟩ (t + 1)) k) #[]).1).enqueue
        (appendFold (fun b : (src : Member L mem) × List WireMsg =>
          if (⟨c, j⟩ : Member L mem) = b.1 then
            filterMapB (fun dx => if dx.1 = addr i.1 then some dx.2 else none) (decodeAt cod b.2 base j.val)
          else #[]) [⟨who k, sent k (who k)⟩]) = _
    rw [appendFold_singleton, traceSched_cursor who sent pc hpc]
    show (((s i j).step _ #[]).1).enqueue
        (if (⟨c, j⟩ : Member L mem) = who k then
          filterMapB (fun dx => if dx.1 = addr i.1 then some dx.2 else none) (decodeAt cod (sent k (who k)) base j.val)
        else #[]) = _
    by_cases hs : (⟨c, j⟩ : Member L mem) = who k
    · rw [if_pos hs]
      exact chan_active_sender who sent pc hpc (dxSel cod base addr i.1 j.val) ⟨c, j⟩ k hs.symm _
        (decodeAt_filterMap_eq _ _ _ _ _ _) (s i j) (h i j)
    · rw [if_neg hs, ChanCell.enqueue_nil]
      exact chan_active_other who sent pc hpc (dxSel cod base addr i.1 j.val) ⟨c, j⟩ k (Ne.symm hs) (s i j) (h i j)
  | _, _, .source_interval .., _, _, _, s, h => fun i => by
    show (if tick k then s i + 1 else s i) = _
    rw [h i, ticksOf_succ]
    cases htk : tick k <;> simp [htk]
  | _, _, .time .., _, _, _, s, h => fun i => by
    show (if tick k then s i + 1 else s i) = _
    rw [h i, ticksOf_succ]
    cases htk : tick k <;> simp [htk]
  | _, _, .call N _, cod, base, _, s, h => Dag.traceQueues_active readings k N cod base _ s h
  | _, _, .fixN _ body, cod, base, _, s, h => Dag.traceQueues_active readings k body cod base _ s h
  | _, _, .map .., _, _, _, _, _ => trivial
  | _, _, .filterMap .., _, _, _, _, _ => trivial
  | _, _, .values .., _, _, _, _, _ => trivial
  | _, _, .into_keyed .., _, _, _, _, _ => trivial
  | _, _, .kassume_ordering .., _, _, _, _, _ => trivial
  | _, _, .kfold_early_stop .., _, _, _, _, _ => trivial
  | _, _, .kget_max_key .., _, _, _, _, _ => trivial
  | _, _, .weaken_retries .., _, _, _, _, _ => trivial
  | _, _, .union .., _, _, _, _, _ => trivial
  | _, _, .assume_ordering .., _, _, _, _, _ => trivial
  | _, _, .fold .., _, _, _, _, _ => trivial
  | _, _, .fold_monotone .., _, _, _, _, _ => trivial
  | _, _, .snapshot .., _, _, _, _, _ => trivial
  | _, _, .batch .., _, _, _, _, _ => trivial
  | _, _, .mapTick .., _, _, _, _, _ => trivial
  | _, _, .firstTick .., _, _, _, _, _ => trivial
  | _, _, .zipTick .., _, _, _, _, _ => trivial
  | _, _, .defer_tick .., _, _, _, _, _ => trivial
  | _, _, .allTicks .., _, _, _, _, _ => trivial
  | _, _, .flattenOrdered .., _, _, _, _, _ => trivial
  | _, _, .flattenUnordered .., _, _, _, _, _ => trivial
  | _, _, .tinto_keyed .., _, _, _, _, _ => trivial
  | _, _, .tassume_ordering .., _, _, _, _, _ => trivial
  | _, _, .kentries .., _, _, _, _, _ => trivial
  | _, _, .kmap .., _, _, _, _, _ => trivial
  | _, _, .kallTicks .., _, _, _, _, _ => trivial
  | _, _, .kfold .., _, _, _, _, _ => trivial
  | _, _, .ksnapshot .., _, _, _, _, _ => trivial
  | _, _, .tfold .., _, _, _, _, _ => trivial
  | _, _, .latest .., _, _, _, _, _ => trivial
  | _, _, .latest_monotone .., _, _, _, _, _ => trivial
  | _, _, .snapshot_bounded .., _, _, _, _, _ => trivial
  | _, _, .reduce_watermark .., _, _, _, _, _ => trivial
  | _, _, .tick_scan .., _, _, _, _, _ => trivial

/-- `Node.traceQueues_active` along the `letN` chain. -/
theorem Dag.traceQueues_active (readings : (k : Nat) → TickReadings (who k)) (k : Nat) :
    {Γ : Ctx L mem} → {outs : List (Kind L mem)} → (d : Dag Γ outs) → (cod : Dag.Codecs d) → (base : Nat) →
    (Δ : HList (Kind.locDelta (who k)) Γ) → (s : Dag.LocState (who k) d) →
    Dag.TraceQueues who sent pc tick k (who k) d cod base s →
    Dag.TraceQueues who sent pc tick (k + 1) (who k) d cod base
      (Dag.enqueueInbox (who k) d (Dag.inboxOfWire (who k) d cod base [⟨who k, sent k (who k)⟩]).1
        (Dag.locStep (who k) (tick k) d (Dag.cursorsAt (who k) d (Dag.traceSched who sent pc tick readings d cod base) k s) Δ s).1)
  | _, _, .ret _, _, _, _, _, _ => trivial
  | _, _, .letN n rest, cod, base, Δ, s, h => by
    refine ⟨?_, ?_⟩
    · show Node.TraceQueues who sent pc tick (k + 1) (who k) n cod.1 base
        (Node.enqueueInbox (who k) n (Node.inboxOfWire (who k) n cod.1 base _).1
          (Node.memoState (who k) n (Node.locStep (who k) (tick k) n _ Δ s.1).1))
      rw [Node.memoState_eq]
      exact Node.traceQueues_active readings k n cod.1 base Δ s.1 h.1
    · show Dag.TraceQueues who sent pc tick (k + 1) (who k) rest cod.2 _
        (Dag.enqueueInbox (who k) rest (Dag.inboxOfWire (who k) rest cod.2 (Node.inboxOfWire (who k) n cod.1 base _).2 _).1
          (Dag.locStep (who k) (tick k) rest _ _ s.2).1)
      rw [Node.inboxOfWire_snd_eq]
      exact Dag.traceQueues_active readings k rest cod.2 _ _ s.2 h.2

end

/-! ### What the running process is handed: the parsed batches, this tick's readings -/

mutual

/-- **Arrivals**: at its activation `k`, what the induced cursors deliver to the running process
(`Node.arrivalsOf`) is `Node.inboxOfWire` of the batches it parsed, listed over the distinct
senders `srcs` it read (`hnd`); a sender not read parses nothing (`hout`). -/
theorem Node.traceQueues_arrivals (readings : (k : Nat) → TickReadings (who k)) (k : Nat)
    (srcs : List (Member L mem)) (hnd : srcs.Nodup)
    (hout : ∀ src, src ∉ srcs → pc (k + 1) (who k) src = pc k (who k) src) :
    {Γ : Ctx L mem} → {κs : List (Kind L mem)} → (n : Node Γ κs) → (cod : Node.Codecs n) → (base : Nat) →
    (s : Node.LocState (who k) n) → Node.TraceQueues who sent pc tick k (who k) n cod base s →
    Node.arrivalsOf (who k) n (Node.cursorsAt (who k) n (Node.traceSched who sent pc tick readings n cod base) k s) s
      = (Node.inboxOfWire (who k) n cod base (srcs.map fun src =>
          ⟨src, ((streamOf who sent (who k) src k).drop (pc k (who k) src)).take
            (pc (k + 1) (who k) src - pc k (who k) src)⟩)).1
  | _, _, .broadcast_closed (c := c) _ _, cod, base, s, h => by
    funext i j
    show (s i j).pending.extract 0 (cumMax (fun t => ccount who sent pc (bcSel cod base j.val) ⟨_, i.1⟩ ⟨c, j⟩ (t + 1)) k
        - (s i j).delivered) = _
    rw [traceSched_cursor who sent pc hpc, chan_arrivals who sent pc hpc _ _ k (s i j) (h i j)]
    show _ = appendFold (fun (b : (src : Member L mem) × List WireMsg) =>
      if (⟨c, j⟩ : Member L mem) = b.1 then decodeAt cod b.2 base j.val else #[]) (srcs.map _)
    rw [appendFold_batches srcs hnd _ (fun l => decodeAt cod l base j.val) rfl ⟨c, j⟩
        (fun hs => by rw [hout _ hs, Nat.sub_self, List.take_zero])]
    apply Array.toList_inj.mp
    rw [List.toList_toArray]
    exact (decodeAt_eq_filterMap _ _ _ _).symm
  | _, _, .demux (c := c) _ _ addr, cod, base, s, h => by
    funext i j
    show (s i j).pending.extract 0 (cumMax (fun t => ccount who sent pc (dxSel cod base addr i.1 j.val) ⟨_, i.1⟩ ⟨c, j⟩ (t + 1)) k
        - (s i j).delivered) = _
    rw [traceSched_cursor who sent pc hpc, chan_arrivals who sent pc hpc _ _ k (s i j) (h i j)]
    show _ = appendFold (fun (b : (src : Member L mem) × List WireMsg) => if (⟨c, j⟩ : Member L mem) = b.1 then
      filterMapB (fun dx => if dx.1 = addr i.1 then some dx.2 else none) (decodeAt cod b.2 base j.val) else #[]) (srcs.map _)
    rw [appendFold_batches srcs hnd _
        (fun l => filterMapB (fun dx => if dx.1 = addr i.1 then some dx.2 else none) (decodeAt cod l base j.val))
        (by
          apply Array.toList_inj.mp
          rw [toList_filterMapB]
          show List.filterMap _ (filterMapToArray _ []).toList = []
          rw [toList_filterMapToArray]
          rfl) ⟨c, j⟩
        (fun hs => by rw [hout _ hs, Nat.sub_self, List.take_zero])]
    apply Array.toList_inj.mp
    rw [List.toList_toArray]
    exact (decodeAt_filterMap_eq _ _ _ _ _ _).symm
  | _, _, .call N _, cod, base, s, h => Dag.traceQueues_arrivals readings k srcs hnd hout N cod base s h
  | _, _, .fixN _ body, cod, base, s, h => Dag.traceQueues_arrivals readings k srcs hnd hout body cod base s h
  | _, _, .source_interval .., _, _, _, _ => rfl
  | _, _, .time .., _, _, _, _ => rfl
  | _, _, .map .., _, _, _, _ => rfl
  | _, _, .filterMap .., _, _, _, _ => rfl
  | _, _, .values .., _, _, _, _ => rfl
  | _, _, .into_keyed .., _, _, _, _ => rfl
  | _, _, .kassume_ordering .., _, _, _, _ => rfl
  | _, _, .kfold_early_stop .., _, _, _, _ => rfl
  | _, _, .kget_max_key .., _, _, _, _ => rfl
  | _, _, .weaken_retries .., _, _, _, _ => rfl
  | _, _, .union .., _, _, _, _ => rfl
  | _, _, .assume_ordering .., _, _, _, _ => rfl
  | _, _, .fold .., _, _, _, _ => rfl
  | _, _, .fold_monotone .., _, _, _, _ => rfl
  | _, _, .snapshot .., _, _, _, _ => rfl
  | _, _, .batch .., _, _, _, _ => rfl
  | _, _, .mapTick .., _, _, _, _ => rfl
  | _, _, .firstTick .., _, _, _, _ => rfl
  | _, _, .zipTick .., _, _, _, _ => rfl
  | _, _, .defer_tick .., _, _, _, _ => rfl
  | _, _, .allTicks .., _, _, _, _ => rfl
  | _, _, .flattenOrdered .., _, _, _, _ => rfl
  | _, _, .flattenUnordered .., _, _, _, _ => rfl
  | _, _, .tinto_keyed .., _, _, _, _ => rfl
  | _, _, .tassume_ordering .., _, _, _, _ => rfl
  | _, _, .kentries .., _, _, _, _ => rfl
  | _, _, .kmap .., _, _, _, _ => rfl
  | _, _, .kallTicks .., _, _, _, _ => rfl
  | _, _, .kfold .., _, _, _, _ => rfl
  | _, _, .ksnapshot .., _, _, _, _ => rfl
  | _, _, .tfold .., _, _, _, _ => rfl
  | _, _, .latest .., _, _, _, _ => rfl
  | _, _, .latest_monotone .., _, _, _, _ => rfl
  | _, _, .snapshot_bounded .., _, _, _, _ => rfl
  | _, _, .reduce_watermark .., _, _, _, _ => rfl
  | _, _, .tick_scan .., _, _, _, _ => rfl

/-- `Node.traceQueues_arrivals` along the `letN` chain. -/
theorem Dag.traceQueues_arrivals (readings : (k : Nat) → TickReadings (who k)) (k : Nat)
    (srcs : List (Member L mem)) (hnd : srcs.Nodup)
    (hout : ∀ src, src ∉ srcs → pc (k + 1) (who k) src = pc k (who k) src) :
    {Γ : Ctx L mem} → {outs : List (Kind L mem)} → (d : Dag Γ outs) → (cod : Dag.Codecs d) → (base : Nat) →
    (s : Dag.LocState (who k) d) → Dag.TraceQueues who sent pc tick k (who k) d cod base s →
    Dag.arrivalsOf (who k) d (Dag.cursorsAt (who k) d (Dag.traceSched who sent pc tick readings d cod base) k s) s
      = (Dag.inboxOfWire (who k) d cod base (srcs.map fun src =>
          ⟨src, ((streamOf who sent (who k) src k).drop (pc k (who k) src)).take
            (pc (k + 1) (who k) src - pc k (who k) src)⟩)).1
  | _, _, .ret _, _, _, _, _ => rfl
  | _, _, .letN n rest, cod, base, s, h => by
    show (Node.arrivalsOf (who k) n (Node.cursorsAt (who k) n (Node.traceSched who sent pc tick readings n cod.1 base) k s.1) s.1,
        Dag.arrivalsOf (who k) rest (Dag.cursorsAt (who k) rest (Dag.traceSched who sent pc tick readings rest cod.2 _) k s.2) s.2)
      = ((Node.inboxOfWire (who k) n cod.1 base _).1, (Dag.inboxOfWire (who k) rest cod.2 (Node.inboxOfWire (who k) n cod.1 base _).2 _).1)
    rw [Node.traceQueues_arrivals readings k srcs hnd hout n cod.1 base s.1 h.1, Node.inboxOfWire_snd_eq,
      Dag.traceQueues_arrivals readings k srcs hnd hout rest cod.2 _ s.2 h.2]

end

omit hpc in
mutual

/-- **Timing**: at a tick `k` of the running process (`htk`), the induced scripts read at its tick
counts (`Node.timingAt`) are the readings it took (`Node.Timing.ofReadings`), because the tick
count under the invariant is `ticksOf`'s length and `nthTick_count'` finds `k` there. -/
theorem Node.traceQueues_timing (readings : (k : Nat) → TickReadings (who k)) (k : Nat) (htk : tick k = true) :
    {Γ : Ctx L mem} → {κs : List (Kind L mem)} → (n : Node Γ κs) → (cod : Node.Codecs n) → (base : Nat) →
    (s : Node.LocState (who k) n) → Node.TraceQueues who sent pc tick k (who k) n cod base s →
    Node.timingAt (who k) n (Node.traceSched who sent pc tick readings n cod base) (Node.arrivedOnly (who k) n s)
      = Node.Timing.ofReadings (who k) (readings k) n
  | _, _, .source_interval (ℓ := ℓ) delay period, _, _, s, h => by
    funext i
    have hi : (ticksOf who tick (who k) k).length = (ticksOf who tick ⟨ℓ, i.1⟩ k).length := by rw [i.2]
    simp only [Node.timingAt, Node.traceSched, Node.arrivedOnly, Node.Timing.ofReadings]
    rw [h i, hi, nthTick_count' who tick k htk i.2.symm]
  | _, _, .time (ℓ := ℓ), _, _, s, h => by
    funext i
    have hi : (ticksOf who tick (who k) k).length = (ticksOf who tick ⟨ℓ, i.1⟩ k).length := by rw [i.2]
    simp only [Node.timingAt, Node.traceSched, Node.arrivedOnly, Node.Timing.ofReadings]
    rw [h i, hi, nthTick_count' who tick k htk i.2.symm]
  | _, _, .call N _, cod, base, s, h => Dag.traceQueues_timing readings k htk N cod base s h
  | _, _, .fixN _ body, cod, base, s, h => Dag.traceQueues_timing readings k htk body cod base s h
  | _, _, .broadcast_closed .., _, _, _, _ => rfl
  | _, _, .demux .., _, _, _, _ => rfl
  | _, _, .map .., _, _, _, _ => rfl
  | _, _, .filterMap .., _, _, _, _ => rfl
  | _, _, .values .., _, _, _, _ => rfl
  | _, _, .into_keyed .., _, _, _, _ => rfl
  | _, _, .kassume_ordering .., _, _, _, _ => rfl
  | _, _, .kfold_early_stop .., _, _, _, _ => rfl
  | _, _, .kget_max_key .., _, _, _, _ => rfl
  | _, _, .weaken_retries .., _, _, _, _ => rfl
  | _, _, .union .., _, _, _, _ => rfl
  | _, _, .assume_ordering .., _, _, _, _ => rfl
  | _, _, .fold .., _, _, _, _ => rfl
  | _, _, .fold_monotone .., _, _, _, _ => rfl
  | _, _, .snapshot .., _, _, _, _ => rfl
  | _, _, .batch .., _, _, _, _ => rfl
  | _, _, .mapTick .., _, _, _, _ => rfl
  | _, _, .firstTick .., _, _, _, _ => rfl
  | _, _, .zipTick .., _, _, _, _ => rfl
  | _, _, .defer_tick .., _, _, _, _ => rfl
  | _, _, .allTicks .., _, _, _, _ => rfl
  | _, _, .flattenOrdered .., _, _, _, _ => rfl
  | _, _, .flattenUnordered .., _, _, _, _ => rfl
  | _, _, .tinto_keyed .., _, _, _, _ => rfl
  | _, _, .tassume_ordering .., _, _, _, _ => rfl
  | _, _, .kentries .., _, _, _, _ => rfl
  | _, _, .kmap .., _, _, _, _ => rfl
  | _, _, .kallTicks .., _, _, _, _ => rfl
  | _, _, .kfold .., _, _, _, _ => rfl
  | _, _, .ksnapshot .., _, _, _, _ => rfl
  | _, _, .tfold .., _, _, _, _ => rfl
  | _, _, .latest .., _, _, _, _ => rfl
  | _, _, .latest_monotone .., _, _, _, _ => rfl
  | _, _, .snapshot_bounded .., _, _, _, _ => rfl
  | _, _, .reduce_watermark .., _, _, _, _ => rfl
  | _, _, .tick_scan .., _, _, _, _ => rfl

/-- `Node.traceQueues_timing` along the `letN` chain. -/
theorem Dag.traceQueues_timing (readings : (k : Nat) → TickReadings (who k)) (k : Nat) (htk : tick k = true) :
    {Γ : Ctx L mem} → {outs : List (Kind L mem)} → (d : Dag Γ outs) → (cod : Dag.Codecs d) → (base : Nat) →
    (s : Dag.LocState (who k) d) → Dag.TraceQueues who sent pc tick k (who k) d cod base s →
    Dag.timingAt (who k) d (Dag.traceSched who sent pc tick readings d cod base) (Dag.arrivedOnly (who k) d s)
      = Dag.Timing.ofReadings (who k) (readings k) d
  | _, _, .ret _, _, _, _, _ => rfl
  | _, _, .letN n rest, cod, base, s, h => by
    show (Node.timingAt (who k) n (Node.traceSched who sent pc tick readings n cod.1 base) (Node.arrivedOnly (who k) n s.1),
        Dag.timingAt (who k) rest (Dag.traceSched who sent pc tick readings rest cod.2 _) (Dag.arrivedOnly (who k) rest s.2))
      = (Node.Timing.ofReadings (who k) (readings k) n, Dag.Timing.ofReadings (who k) (readings k) rest)
    rw [Node.traceQueues_timing readings k htk n cod.1 base s.1 h.1, Dag.traceQueues_timing readings k htk rest cod.2 _ s.2 h.2]

end

end Induced

/-! ## The trace as an interleaving -/

namespace PassTrace

variable {Γ : Ctx L mem} {outs : List (Kind L mem)} {d : Dag Γ outs}

/-- The trace's parsed counts satisfy `PcOk` (`pc_succ_other`, `pc_le`, `pc_mono`,
`pc_succ_self_le`). -/
theorem pcOk (cod : Dag.Codecs d) (τ : PassTrace d) : PcOk τ.who (τ.sent cod) (τ.pc cod) :=
  { idle := fun k m src hm => τ.pc_succ_other cod k m src hm
    le := fun k m src => τ.pc_le cod k m src
    mono := fun k m src => τ.pc_mono cod k m src
    self_le := fun k src => τ.pc_succ_self_le cod k src }

/-- The schedule the trace induces (`Dag.traceSched` at wire base `0`): per network node, per
receiver and sender, the cursor at step `t` is the number of that connection's messages the
receiver has parsed through `t`; per timer node, the script is the readings indexed by tick. -/
noncomputable def sched (cod : Dag.Codecs d) (τ : PassTrace d) : Dag.Sched d :=
  Dag.traceSched τ.who (τ.sent cod) (τ.pc cod) τ.tick τ.readings d cod 0

/-- The interleaving a `PassTrace` induces: its processes, tick flags and inputs, with the
induced schedule. -/
noncomputable def interleaving (cod : Dag.Codecs d) (τ : PassTrace d) : Interleaving d :=
  ⟨τ.who, τ.tick, τ.inp, τ.sched cod⟩

/-- **The invariant the induction carries** at step `k`: every member's lock-step view satisfies
`TraceQueues`, the trace's cells are the model's loop cells, and every earlier activation's
bytes are the model's `wireOut`. -/
structure Inv (cod : Dag.Codecs d) (τ : PassTrace d) (k : Nat) : Prop where
  queues : ∀ m, Dag.TraceQueues τ.who (τ.sent cod) (τ.pc cod) τ.tick k m d cod 0 ((τ.interleaving cod).view k m)
  cells : ∀ m, τ.cells cod k m = (τ.interleaving cod).loopView k m
  sends : ∀ k' < k, τ.sends cod k' = (τ.interleaving cod).wireOut cod k'

/-- Under `Inv` at `k`, activation `k` as `Dag.pass` on the parsed batches with the readings as
timing equals the model's `loopPass`. The arrivals match by `Dag.traceQueues_arrivals`; the timing
matches at a tick by `Dag.traceQueues_timing`, and at a stutter by `Dag.pass_false_timing`. -/
theorem pass_eq_of_inv (cod : Dag.Codecs d) (τ : PassTrace d) (k : Nat) (h : τ.Inv cod k) :
    Dag.pass (τ.who k) (τ.tick k) d (Dag.Timing.ofReadings (τ.who k) (τ.readings k) d)
        (Dag.inboxOfWire (τ.who k) d cod 0 (τ.batches cod k)).1 (envRestrict (τ.who k) (τ.inp k)) (τ.cells cod k (τ.who k))
      = (τ.interleaving cod).loopPass k := by
  unfold Interleaving.loopPass
  rw [h.cells]
  -- the arrivals: the induced cursors deliver the batches the trace parsed
  have ha : (τ.interleaving cod).arrivals k = (Dag.inboxOfWire (τ.who k) d cod 0 (τ.batches cod k)).1 := by
    show Dag.arrivalsOf (τ.who k) d
      (Dag.cursorsAt (τ.who k) d (Dag.traceSched τ.who (τ.sent cod) (τ.pc cod) τ.tick τ.readings d cod 0) k
        ((τ.interleaving cod).view k (τ.who k)))
      ((τ.interleaving cod).view k (τ.who k)) = _
    rw [τ.batches_eq cod k]
    exact Dag.traceQueues_arrivals τ.who (τ.sent cod) (τ.pc cod) τ.tick (τ.pcOk cod) τ.readings k _ (srcsOf_nodup _)
      (fun src hs => τ.pc_succ_self_of_not_mem cod k src hs) d cod 0 _ (h.queues (τ.who k))
  rw [ha]
  show Dag.pass (τ.who k) (τ.tick k) d (Dag.Timing.ofReadings (τ.who k) (τ.readings k) d) _
      (envRestrict (τ.who k) (τ.inp k)) ((τ.interleaving cod).loopView k (τ.who k))
    = Dag.pass (τ.who k) (τ.tick k) d ((τ.interleaving cod).timing k) _
      (envRestrict (τ.who k) (τ.inp k)) ((τ.interleaving cod).loopView k (τ.who k))
  -- the timing: at a tick the induced scripts read the readings; a stutter reads no timing
  cases htk : τ.tick k with
  | true =>
    have ht : (τ.interleaving cod).timing k = Dag.Timing.ofReadings (τ.who k) (τ.readings k) d := by
      show Dag.timingAt (τ.who k) d (Dag.traceSched τ.who (τ.sent cod) (τ.pc cod) τ.tick τ.readings d cod 0)
        (Dag.arrivedOnly (τ.who k) d ((τ.interleaving cod).view k (τ.who k))) = _
      exact Dag.traceQueues_timing τ.who (τ.sent cod) (τ.pc cod) τ.tick τ.readings k htk d cod 0 _ (h.queues (τ.who k))
    rw [ht]
  | false =>
    exact Dag.pass_false_timing (τ.who k) d _ _ _ _ _

/-- `Inv` at step `0`: `Dag.traceQueues_init`, `initCells`, no sends yet. -/
theorem inv_zero (cod : Dag.Codecs d) (τ : PassTrace d) : τ.Inv cod 0 :=
  { queues := fun m => Dag.traceQueues_init τ.who (τ.sent cod) (τ.pc cod) τ.tick m d cod 0
    cells := fun _ => rfl
    sends := fun _ h => absurd h (Nat.not_lt_zero _) }

/-- `Inv` is preserved by a step. `pass_eq_of_inv` makes activation `k` the model's `loopPass`,
so its sends are `wireOut`; the queues move by `Dag.traceQueues_active` at the running process
(`view_active_wire`) and `Dag.traceQueues_idle` elsewhere (`view_idle_wire`); the cells by
`loop_pass` and `loop_idle`. -/
theorem inv_succ (cod : Dag.Codecs d) (τ : PassTrace d) (k : Nat) (h : τ.Inv cod k) : τ.Inv cod (k + 1) := by
  have hact := τ.pass_eq_of_inv cod k h
  have hloop := (τ.interleaving cod).loop_pass k
  have hsend : τ.sends cod k = (τ.interleaving cod).wireOut cod k := by
    rw [sends_eq, act_eq_wire]
    show (Dag.wireEncode (τ.who k) d cod 0 (Dag.pass (τ.who k) (τ.tick k) d _ (Dag.inboxOfWire (τ.who k) d cod 0 (τ.batches cod k)).1 _ _).2.2).1 = _
    rw [hact, hloop]
    rfl
  have hsent : τ.sent cod k = fun m => Dag.wireFor m d ((τ.interleaving cod).wireOut cod k) := by
    funext m; unfold sent; rw [hsend]
  refine ⟨fun m => ?_, fun m => ?_, fun k' hk' => ?_⟩
  · by_cases hm : τ.who k = m
    · subst hm
      show Dag.TraceQueues τ.who (τ.sent cod) (τ.pc cod) τ.tick (k + 1) (τ.who k) d cod 0
        ((τ.interleaving cod).view (k + 1) ((τ.interleaving cod).who k))
      rw [(τ.interleaving cod).view_active_wire cod k]
      have := Dag.traceQueues_active τ.who (τ.sent cod) (τ.pc cod) τ.tick (τ.pcOk cod) τ.readings k d cod 0
        (envRestrict (τ.who k) (τ.inp k)) _ (h.queues (τ.who k))
      rw [hsent] at this
      exact this
    · show Dag.TraceQueues τ.who (τ.sent cod) (τ.pc cod) τ.tick (k + 1) m d cod 0 ((τ.interleaving cod).view (k + 1) m)
      rw [(τ.interleaving cod).view_idle_wire cod k m (Ne.symm hm)]
      have := Dag.traceQueues_idle τ.who (τ.sent cod) (τ.pc cod) τ.tick (τ.pcOk cod) k m hm d cod 0 _ (h.queues m)
      rw [hsent] at this
      exact this
  · by_cases hm : τ.who k = m
    · subst hm
      rw [cells_succ_self, act_eq_wire]
      show (Dag.pass (τ.who k) (τ.tick k) d _ (Dag.inboxOfWire (τ.who k) d cod 0 (τ.batches cod k)).1 _ _).1 = _
      rw [hact, hloop]
      rfl
    · rw [cells_succ_other cod τ k m hm, (τ.interleaving cod).loop_idle k m (Ne.symm hm), h.cells m]
  · rcases Nat.lt_or_eq_of_le (Nat.lt_succ_iff.mp hk') with hlt | rfl
    · exact h.sends k' hlt
    · exact hsend

/-- `Inv` holds at every step. -/
theorem inv (cod : Dag.Codecs d) (τ : PassTrace d) : ∀ k, τ.Inv cod k
  | 0 => τ.inv_zero cod
  | k + 1 => τ.inv_succ cod k (τ.inv cod k)

/-- **The trace's cells are the model's loop cells**, at every step and member. -/
theorem cells_eq (cod : Dag.Codecs d) (τ : PassTrace d) (k : Nat) (m : Member L mem) :
    τ.cells cod k m = (τ.interleaving cod).loopView k m := (τ.inv cod k).cells m

/-- **The induced interleaving is batched by the parsed frames**: its arrivals at step `k` decode
from the batches activation `k` parsed (`Dag.traceQueues_arrivals` under `inv`). -/
theorem arrivals_eq (cod : Dag.Codecs d) (τ : PassTrace d) (k : Nat) :
    (τ.interleaving cod).arrivals k = (Dag.inboxOfWire (τ.who k) d cod 0 (τ.batches cod k)).1 := by
  have h := τ.inv cod k
  show Dag.arrivalsOf (τ.who k) d
    (Dag.cursorsAt (τ.who k) d (Dag.traceSched τ.who (τ.sent cod) (τ.pc cod) τ.tick τ.readings d cod 0) k
      ((τ.interleaving cod).view k (τ.who k)))
    ((τ.interleaving cod).view k (τ.who k)) = _
  rw [τ.batches_eq cod k]
  exact Dag.traceQueues_arrivals τ.who (τ.sent cod) (τ.pc cod) τ.tick (τ.pcOk cod) τ.readings k _ (srcsOf_nodup _)
    (fun src hs => τ.pc_succ_self_of_not_mem cod k src hs) d cod 0 _ (h.queues (τ.who k))

/-- The induced interleaving as a `Batched` interleaving, with the trace's `batches`. -/
noncomputable def batched (cod : Dag.Codecs d) (τ : PassTrace d) : (τ.interleaving cod).Batched cod :=
  ⟨τ.batches cod, τ.arrivals_eq cod⟩

/-- The model's timing at a tick step `k` (`htk`) is the readings the trace took. -/
theorem timing_eq (cod : Dag.Codecs d) (τ : PassTrace d) (k : Nat) (htk : τ.tick k = true) :
    (τ.interleaving cod).timing k = Dag.Timing.ofReadings (τ.who k) (τ.readings k) d := by
  show Dag.timingAt (τ.who k) d (Dag.traceSched τ.who (τ.sent cod) (τ.pc cod) τ.tick τ.readings d cod 0)
    (Dag.arrivedOnly (τ.who k) d ((τ.interleaving cod).view k (τ.who k))) = _
  exact Dag.traceQueues_timing τ.who (τ.sent cod) (τ.pc cod) τ.tick τ.readings k htk d cod 0 _ ((τ.inv cod k).queues (τ.who k))

/-- **The trace's run matches the model's run at the wire**: every activation's cells, outputs
and bytes equal the batched interleaving's `loopPassWire`. -/
theorem act_eq (cod : Dag.Codecs d) (τ : PassTrace d) (k : Nat) :
    ((τ.act cod k).1, (τ.act cod k).2.1, (τ.act cod k).2.2.1) = (τ.batched cod).loopPassWire k := by
  rw [act_eq_wire, (τ.batched cod).loop_pass_wire k]
  show (let r := Dag.pass (τ.who k) (τ.tick k) d _ (Dag.inboxOfWire (τ.who k) d cod 0 (τ.batches cod k)).1 _ _
    (r.1, r.2.1, (Dag.wireEncode (τ.who k) d cod 0 r.2.2).1)) = _
  rw [τ.pass_eq_of_inv cod k (τ.inv cod k), (τ.interleaving cod).loop_pass k]
  rfl

/-- **What a process outputs along a `PassTrace` is what it outputs in the model**: `act_eq` at
each of its activations, through `Batched.streamOutputsAt_eq`. -/
theorem streamOutputsAt_eq (cod : Dag.Codecs d) (τ : PassTrace d) (T : Nat) (m : Member L mem) {ℓ : L} {α : Type} {iα : DecidableEq α} {ord : StrOrd}
    {ret : Retries} (v : Var outs (.stream ℓ α iα ord ret)) (i : Hosted m ℓ) :
    τ.streamOutputsAt cod T m v i = (τ.interleaving cod).streamOutputsAt T m v i := by
  rw [← (τ.batched cod).streamOutputsAt_eq]
  unfold streamOutputsAt Interleaving.Batched.streamOutputsAt Interleaving.Batched.outputsAt
  congr 1
  apply List.filterMap_congr
  intro k _
  show (if h : τ.who k = m then some (h ▸ (τ.act cod k).2.1) else none)
    = if h : τ.who k = m then some (h ▸ ((τ.batched cod).loopPassWire k).2.1) else none
  rw [← τ.act_eq]

end PassTrace

/-! ## The trace: what a deployment produces

**A trace at the bytes** is four functions: which process ran at each activation, the inputs it
was given, the clock and timer readings it took, and how many bytes it read off each of its
connections. The rest follows from TCP and the process's body: a connection is a byte queue
(TCP), a process reads some prefix of what is queued, cut anywhere, mid-frame included, and runs
its **body** on it (`WireTrace.Body`: the interpreter's `Body.ofCodecs cod`, or a staged
activation equal to it); the complete frames are its messages, the incomplete tail is kept, its
sends go on its connections as frames. There is no assumption to state: every
`who`/`inp`/`readings`/`reads` is a trace. On a valid graph the activation equals the pass
(`Dag.iterateBytes_eq_passBytes`), so the trace runs as the `PassTrace` that ticks at every step
(`WireTrace.toPass`), and `WireTrace.streamOutputsAt_eq` says the trace's outputs are the
model's; `paxos_safe_wired` (`Paxos/WiredSafety.lean`) quantifies over traces. -/

/-- A trace at the bytes: who ran at each activation, its inputs, its readings, and the bytes it
read per connection. -/
structure WireTrace {Γ : Ctx L mem} {outs : List (Kind L mem)} (d : Dag Γ outs) where
  who : Nat → Member L mem
  inp : Nat → HList (Kind.delta (mem := mem)) Γ
  readings : (k : Nat) → TickReadings (who k)
  reads : Nat → List (Member L mem × Nat)

namespace WireTrace

variable {Γ : Ctx L mem} {outs : List (Kind L mem)} {d : Dag Γ outs}

/-- **What a process runs at an activation**: a loop body at the bytes for the graph. It takes
the running member, this tick's readings, per connection the bytes (the kept tail and what
arrived), its input increments and its cells; it returns its cells, its output increments, its
sends and the tails to keep. A trace is run by a body (`run`), and the headlines are stated over
the body the processes run: `ofCodecs cod` runs the interpreter's `Dag.iterateBytes`, and a staged
activation (`hydro_run`) is a body equal to it by its generated equation (`paxosBody`,
`Paxos/WiredSafety.lean`). -/
def Body (d : Dag Γ outs) : Type :=
  (m : Member L mem) → TickReadings m → List ((src : Member L mem) × ByteArray) → HList (Kind.locDelta m) Γ →
    Dag.LocState m d → Dag.LocState m d × HList (Kind.locDelta m) outs × List WireMsg × List ByteArray

/-- The interpreter's body: `Dag.iterateBytes` at the codecs, ticking. -/
def Body.ofCodecs (cod : Dag.Codecs d) : Body d := fun m r chans Δ s =>
  Dag.iterateBytes m true d cod (Dag.Timing.ofReadings m r d) chans Δ s

/-- Activation `k` on a run state: the body on the running process's cells, handed each listed
connection's kept tail plus the requested bytes, as many as are there. -/
def actOf (b : Body d) (τ : WireTrace d) (k : Nat) (r : RunState d) :=
  let m := τ.who k
  let S := fun src => PassTrace.bytesOn (d := d) τ.who r.sends m src k
  let cur' := fun src => r.cur m src + min (bytesReq (τ.reads k) src) ((S src).size - r.cur m src)
  let bytes := fun src => r.buf m src ++ (S src).extract (r.cur m src) (cur' src)
  let chans := (srcsOf (τ.reads k)).map fun src => (⟨src, bytes src⟩ : (src : Member L mem) × ByteArray)
  b m (τ.readings k) chans (envRestrict m (τ.inp k)) (r.cells m)

/-- Step `k` of the run: `PassTrace.step` with the body in place of `Dag.passBytes`. -/
def step (b : Body d) (τ : WireTrace d) (k : Nat) (r : RunState d) : RunState d :=
  let m := τ.who k
  let S := fun src => PassTrace.bytesOn (d := d) τ.who r.sends m src k
  let cur' := fun src => r.cur m src + min (bytesReq (τ.reads k) src) ((S src).size - r.cur m src)
  let bytes := fun src => r.buf m src ++ (S src).extract (r.cur m src) (cur' src)
  let a := τ.actOf b k r
  { cells := fun m' => if h : m = m' then h ▸ a.1 else r.cells m'
    cur := fun m' src => if m = m' then cur' src else r.cur m' src
    buf := fun m' src => if m = m' ∧ src ∈ srcsOf (τ.reads k) then (parseFrames (bytes src)).2 else r.buf m' src
    sends := fun k' => if k' = k then a.2.2.1 else r.sends k' }

/-- The run under body `b`, from the same initial state as `PassTrace.run`. -/
def run (b : Body d) (τ : WireTrace d) : Nat → RunState d
  | 0 => ⟨fun m => PassTrace.initCells m, fun _ _ => 0, fun _ _ => .empty, fun _ => []⟩
  | k + 1 => τ.step b k (τ.run b k)

/-- Process `m`'s cells before activation `k`. -/
def cells (b : Body d) (τ : WireTrace d) (k : Nat) (m : Member L mem) : Dag.LocState m d := (τ.run b k).cells m

/-- The messages activation `k` put on the wire. -/
def sends (b : Body d) (τ : WireTrace d) (k : Nat) : List WireMsg := (τ.run b (k + 1)).sends k

/-- Activation `k`: the body on the running process's cells, at the run state before `k`. -/
def act (b : Body d) (τ : WireTrace d) (k : Nat) := τ.actOf b k (τ.run b k)

/-- The items a stream output `v` of process `m` has carried along the trace by activation `T`. -/
def streamOutputsAt (b : Body d) (τ : WireTrace d) (T : Nat) (m : Member L mem) {ℓ : L} {α : Type} {iα : DecidableEq α} {ord : StrOrd}
    {ret : Retries} (v : Var outs (.stream ℓ α iα ord ret)) (i : Hosted m ℓ) : List α :=
  ((List.range T).filterMap fun k => if h : τ.who k = m then some (h ▸ (τ.act b k).2.1) else none).flatMap
    fun o => (o.get v i).toList

/-- The run together with member `m`'s output increments so far, in one pass over the
activations: what evaluating a trace costs (`WireTraceCheck`). `streamOutputsAt_eq_runOutputs`
says its outputs are `streamOutputsAt`. -/
def runOutputs (b : Body d) (τ : WireTrace d) (m : Member L mem) :
    Nat → RunState d × List (HList (Kind.locDelta m) outs)
  | 0 => (τ.run b 0, [])
  | k + 1 =>
    let p := τ.runOutputs b m k
    (τ.step b k p.1, p.2 ++ if h : τ.who k = m then [h ▸ (τ.actOf b k p.1).2.1] else [])

/-- `runOutputs`' run state is `run`. -/
theorem runOutputs_fst (b : Body d) (τ : WireTrace d) (m : Member L mem) :
    ∀ k, (τ.runOutputs b m k).1 = τ.run b k
  | 0 => rfl
  | k + 1 => by
    show τ.step b k (τ.runOutputs b m k).1 = τ.step b k (τ.run b k)
    rw [runOutputs_fst b τ m k]

/-- `runOutputs`' outputs are `m`'s activations' output increments before `k`. -/
theorem runOutputs_snd (b : Body d) (τ : WireTrace d) (m : Member L mem) :
    ∀ k, (τ.runOutputs b m k).2
      = (List.range k).filterMap fun k => if h : τ.who k = m then some (h ▸ (τ.act b k).2.1) else none
  | 0 => rfl
  | k + 1 => by
    show (τ.runOutputs b m k).2 ++ (if h : τ.who k = m then [h ▸ (τ.actOf b k (τ.runOutputs b m k).1).2.1] else []) = _
    rw [List.range_succ, List.filterMap_append, runOutputs_snd b τ m k, runOutputs_fst]
    congr 1
    by_cases h : τ.who k = m
    · simp [h, act]
    · simp [h]

/-- `streamOutputsAt` read off `runOutputs`. -/
theorem streamOutputsAt_eq_runOutputs (b : Body d) (τ : WireTrace d) (T : Nat) (m : Member L mem) {ℓ : L} {α : Type}
    {iα : DecidableEq α} {ord : StrOrd} {ret : Retries} (v : Var outs (.stream ℓ α iα ord ret)) (i : Hosted m ℓ) :
    τ.streamOutputsAt b T m v i = (τ.runOutputs b m T).2.flatMap fun o => (o.get v i).toList := by
  unfold streamOutputsAt
  rw [runOutputs_snd]

/-! ### The trace as a trace of passes: every activation a tick

A valid graph's activation is its pass (`Dag.iterateBytes_eq_passBytes`), so under the
interpreter's body the trace runs as the `PassTrace` that ticks at every step. -/

/-- The pass-level picture of a trace: every activation a tick pass. -/
def toPass (τ : WireTrace d) : PassTrace d := ⟨τ.who, fun _ => true, τ.inp, τ.readings, τ.reads⟩

/-- Under `Body.ofCodecs` on a valid graph (`hv`), an activation is `toPass`'s
(`Dag.iterateBytes_eq_passBytes`). -/
theorem actOf_eq (hv : d.valid = true) (cod : Dag.Codecs d) (τ : WireTrace d) (k : Nat) (r : RunState d) :
    τ.actOf (Body.ofCodecs cod) k r = τ.toPass.actOf cod k r := by
  unfold actOf PassTrace.actOf Body.ofCodecs
  exact Dag.iterateBytes_eq_passBytes _ _ d hv _ _ _ _ _

/-- `actOf_eq` lifted to a step. -/
theorem step_eq (hv : d.valid = true) (cod : Dag.Codecs d) (τ : WireTrace d) (k : Nat) (r : RunState d) :
    τ.step (Body.ofCodecs cod) k r = τ.toPass.step cod k r := by
  unfold step PassTrace.step
  rw [actOf_eq hv]
  rfl

/-- `step_eq` lifted to the run. -/
theorem run_eq (hv : d.valid = true) (cod : Dag.Codecs d) (τ : WireTrace d) :
    ∀ k, τ.run (Body.ofCodecs cod) k = τ.toPass.run cod k
  | 0 => rfl
  | k + 1 => by
    show τ.step (Body.ofCodecs cod) k (τ.run (Body.ofCodecs cod) k) = τ.toPass.step cod k (τ.toPass.run cod k)
    rw [run_eq hv cod τ k, step_eq hv]

/-- Under `Body.ofCodecs` on a valid graph, activation `k` is `toPass`'s `act`. -/
theorem act_eq (hv : d.valid = true) (cod : Dag.Codecs d) (τ : WireTrace d) (k : Nat) :
    τ.act (Body.ofCodecs cod) k = τ.toPass.act cod k := by
  rw [PassTrace.act_eq_actOf]
  show τ.actOf (Body.ofCodecs cod) k (τ.run (Body.ofCodecs cod) k) = _
  rw [run_eq hv, actOf_eq hv]

/-- The lock-step interleaving a wire trace induces: that of `toPass`. -/
noncomputable def interleaving (cod : Dag.Codecs d) (τ : WireTrace d) : Interleaving d :=
  τ.toPass.interleaving cod

/-- **A wire trace's outputs equal the model's**: on a valid graph (`hv`), what a stream output of
process `m` has carried by activation `T` under the interpreter's body is what the induced
interleaving carries by step `T`. `act_eq` reduces it to `PassTrace.streamOutputsAt_eq`. -/
theorem streamOutputsAt_eq (hv : d.valid = true) (cod : Dag.Codecs d) (τ : WireTrace d) (T : Nat) (m : Member L mem) {ℓ : L}
    {α : Type} {iα : DecidableEq α} {ord : StrOrd} {ret : Retries} (v : Var outs (.stream ℓ α iα ord ret)) (i : Hosted m ℓ) :
    τ.streamOutputsAt (Body.ofCodecs cod) T m v i = (τ.interleaving cod).streamOutputsAt T m v i := by
  unfold interleaving
  rw [← τ.toPass.streamOutputsAt_eq cod T m v i]
  unfold streamOutputsAt PassTrace.streamOutputsAt
  congr 1
  apply List.filterMap_congr
  intro k _
  show (if h : τ.who k = m then some (h ▸ (τ.act (Body.ofCodecs cod) k).2.1) else none)
    = if h : τ.who k = m then some (h ▸ (τ.toPass.act cod k).2.1) else none
  rw [act_eq hv]

end WireTrace

end Project

end Hydro
