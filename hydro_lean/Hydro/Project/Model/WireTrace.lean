import Hydro.Project.Model.DistributedModel
import Mathlib.Data.List.Dedup

/-!
# Hydro · Project · Model — a run at the sockets is a run of the model (D88–D90)

**Model, not production.** What a deployment produces is a **trace at the bytes**: a
sequence of activations — which process ran, the inputs it was given, the clock and timer
readings it took, and *how many bytes it read off each of its connections* (`WireTrace`).
Nothing else: a connection is a byte queue (TCP), a process reads some prefix of what is
queued — cut anywhere, mid-frame included — and runs the production body
`Dag.iterateBytes` on it: the complete frames are its messages, the incomplete tail is kept
for the next activation, its sends go on its connections as frames (`WireTrace.run`).
There is no assumption to state: every `who`/`inp`/`readings`/`reads` is a trace. What
stays outside is crash-stop (a process that stops never runs again — it simply has no
further activations) and the sockets themselves.

This file shows **every such trace is a batched interleaving** (`WireTrace.interleaving`,
`WireTrace.batched`): the delivery cursor of a connection is the number of its messages
among the frames the receiver has parsed, the timing scripts are the readings indexed by
activation, and the trace's run is the model's run at the wire (`WireTrace.act_eq`).
`paxos_safe_wired` (`Paxos/WiredSafety.lean`) is then the headline with no interleaving in
its statement: it quantifies over traces.

Two invariants carry the induction. On the run itself (`RunInv`): a receiver's byte
cursor on a connection is within what the sender has written, and its kept tail is the
bytes past the last complete frame (`Wire.splitFrames` says where that is). On the
lock-step cells (`Dag.TraceQueues`): each receiving end's queue holds exactly the
connection's messages past the parsed prefix, in order, and its delivered count is the
message count of the parsed prefix; each timer cell is the member's activation count.
The framing law (`Wire.framesFrom_prefix_frameAll`) is what turns bytes read into
messages parsed: cutting a stream of frames anywhere yields the frames that fit and keeps
the rest.
-/

namespace Hydro

namespace Project

variable {L : Type} {mem : L → Nat} [DecidableEq L]

/-! ## The trace's combinatorics -/

/-- The activations of member `m` before step `K`, in order. -/
def activationsOf (who : Nat → Member L mem) (m : Member L mem) (K : Nat) : List Nat :=
  (List.range K).filter fun k => decide (who k = m)

theorem activationsOf_succ (who : Nat → Member L mem) (m : Member L mem) (K : Nat) :
    activationsOf who m (K + 1) = activationsOf who m K ++ (if who K = m then [K] else []) := by
  unfold activationsOf
  rw [List.range_succ, List.filter_append, List.filter_singleton]
  split <;> simp_all

theorem activationsOf_mono (who : Nat → Member L mem) (m : Member L mem) {K K' : Nat} (h : K ≤ K') :
    activationsOf who m K <+: activationsOf who m K' := by
  induction K' with
  | zero => cases Nat.le_zero.mp h; exact List.prefix_refl _
  | succ K' ih =>
    rcases Nat.lt_or_ge K (K' + 1) with hlt | hge
    · exact (ih (Nat.lt_succ_iff.mp hlt)).trans (by rw [activationsOf_succ]; exact List.prefix_append _ _)
    · cases Nat.le_antisymm h hge; exact List.prefix_refl _

theorem mem_activationsOf {who : Nat → Member L mem} {m : Member L mem} {K k : Nat} :
    k ∈ activationsOf who m K ↔ k < K ∧ who k = m := by
  simp [activationsOf]

/-- Activation counts separate activations: two activations of `m` with the same count are
the same step. -/
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
/-- The `n`-th activation of member `m`, if it has happened, with its witness. -/
noncomputable def nthActivation (who : Nat → Member L mem) (m : Member L mem) (n : Nat) :
    Option {k : Nat // who k = m} :=
  if h : ∃ k, who k = m ∧ (activationsOf who m k).length = n then
    some ⟨Classical.choose h, (Classical.choose_spec h).1⟩
  else none

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

theorem nthActivation_count' (who : Nat → Member L mem) (k : Nat) {m : Member L mem} (h : who k = m) :
    nthActivation who m (activationsOf who m k).length = some ⟨k, h⟩ := by
  subst h
  exact nthActivation_count who k

/-- What a sender has put on its connection to `m` by step `K`: its activations' messages
for `m`, in order (`sent k' m` the messages activation `k'` put on that connection). -/
def streamOf (who : Nat → Member L mem) (sent : Nat → Member L mem → List WireMsg) (m src : Member L mem) (K : Nat) :
    List WireMsg :=
  (activationsOf who src K).flatMap fun k' => sent k' m

theorem streamOf_succ (who : Nat → Member L mem) (sent : Nat → Member L mem → List WireMsg) (m src : Member L mem)
    (K : Nat) :
    streamOf who sent m src (K + 1) = streamOf who sent m src K ++ (if who K = src then sent K m else []) := by
  unfold streamOf
  rw [activationsOf_succ, List.flatMap_append]
  split <;> simp

/-- The bytes read from each connection this activation (`reads`: connection, byte count;
a connection listed twice is read twice). -/
def bytesReq (reads : List (Member L mem × Nat)) (src : Member L mem) : Nat :=
  ((reads.filter fun r => decide (r.1 = src)).map Prod.snd).sum

/-- The connections read this activation, each once. -/
def srcsOf (reads : List (Member L mem × Nat)) : List (Member L mem) := (reads.map Prod.fst).dedup

theorem srcsOf_nodup (reads : List (Member L mem × Nat)) : (srcsOf reads).Nodup := List.nodup_dedup _

theorem bytesReq_eq_zero {reads : List (Member L mem × Nat)} {src : Member L mem} (h : src ∉ srcsOf reads) :
    bytesReq reads src = 0 := by
  unfold bytesReq
  rw [List.sum_eq_zero]
  intro x hx
  obtain ⟨r, hr, rfl⟩ := List.mem_map.mp hx
  have := List.mem_filter.mp hr
  exact absurd (List.mem_dedup.mpr (List.mem_map.mpr ⟨r, this.1, of_decide_eq_true this.2⟩)) h


/-! ### Ticks (D93)

A pass-level trace (`PassTrace`) has ticks and stutters; a member's timer cells count its
ticks, so the induced scripts index by tick count. -/

/-- The ticks of member `m` before step `K`, in order. -/
def ticksOf (who : Nat → Member L mem) (tick : Nat → Bool) (m : Member L mem) (K : Nat) : List Nat :=
  (List.range K).filter fun k => decide (who k = m) && tick k

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

theorem ticksOf_mono (who : Nat → Member L mem) (tick : Nat → Bool) (m : Member L mem) {K K' : Nat} (h : K ≤ K') :
    ticksOf who tick m K <+: ticksOf who tick m K' := by
  induction K' with
  | zero => cases Nat.le_zero.mp h; exact List.prefix_refl _
  | succ K' ih =>
    rcases Nat.lt_or_ge K (K' + 1) with hlt | hge
    · exact (ih (Nat.lt_succ_iff.mp hlt)).trans (by rw [ticksOf_succ]; exact List.prefix_append _ _)
    · cases Nat.le_antisymm h hge; exact List.prefix_refl _

theorem mem_ticksOf {who : Nat → Member L mem} {tick : Nat → Bool} {m : Member L mem} {K k : Nat} :
    k ∈ ticksOf who tick m K ↔ k < K ∧ who k = m ∧ tick k = true := by
  simp [ticksOf]

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
/-- The `n`-th tick of member `m`, if it has happened, with its witness. -/
noncomputable def nthTick (who : Nat → Member L mem) (tick : Nat → Bool) (m : Member L mem) (n : Nat) :
    Option {k : Nat // who k = m} :=
  if h : ∃ k, who k = m ∧ tick k = true ∧ (ticksOf who tick m k).length = n then
    some ⟨Classical.choose h, (Classical.choose_spec h).1⟩
  else none

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

theorem nthTick_count' (who : Nat → Member L mem) (tick : Nat → Bool) (k : Nat) (htk : tick k = true) {m : Member L mem}
    (h : who k = m) : nthTick who tick m (ticksOf who tick m k).length = some ⟨k, h⟩ := by
  subst h
  exact nthTick_count who tick k htk

/-! ## The pass-level trace (the proof layer's object)

`PassTrace` is the model's picture of a run: one *pass* (a tick or a stutter) per step.
The trusted object — a `PassTrace`, one activation per step, each activation the
production body `Dag.passBytes` (a tick, then stutters until quiet) — is below
(`## The trace`); a `PassTrace` expands to a `PassTrace` (`PassTrace.expand`), and every
`PassTrace` is a batched interleaving. -/

/-- **A trace at the bytes**: who ran at each step, the inputs it was given, the clock and
timer readings it took, and how many bytes it read off each of its connections. -/
structure PassTrace {Γ : Ctx L mem} {outs : List (Kind L mem)} (d : Dag Γ outs) where
  who : Nat → Member L mem
  /-- Whether the pass ticks (else it stutters: no tick body, no timer, the knots' feedback released). -/
  tick : Nat → Bool
  inp : Nat → HList (Kind.delta (mem := mem)) Γ
  readings : (k : Nat) → TickReadings (who k)
  reads : Nat → List (Member L mem × Nat)

/-- The run so far: every process's cells, per connection the receiver's byte cursor and
kept tail, and what each earlier activation put on the wire. -/
structure RunState {Γ : Ctx L mem} {outs : List (Kind L mem)} (d : Dag Γ outs) where
  cells : (m : Member L mem) → Dag.LocState m d
  cur : Member L mem → Member L mem → Nat
  buf : Member L mem → Member L mem → ByteArray
  sends : Nat → List WireMsg

namespace PassTrace

variable {Γ : Ctx L mem} {outs : List (Kind L mem)} {d : Dag Γ outs}

/-- A process's cells before its first activation. -/
def initCells (m : Member L mem) : Dag.LocState m d := Dag.arrivedOnly m d (Dag.restrict m d (Dag.init d))

/-- The bytes `src` has put on its connection to `m` by step `k`, given the activations'
sends so far. -/
def bytesOn (who : Nat → Member L mem) (sends : Nat → List WireMsg) (m src : Member L mem) (k : Nat) : ByteArray :=
  frameMsgs (streamOf who (fun k' m' => Dag.wireFor m' d (sends k')) m src k)

/-- Activation `k` on a run state: the body at the bytes on the running process's cells,
handed each listed connection's kept tail plus the requested bytes (as many as are there). -/
def actOf (cod : Dag.Codecs d) (τ : PassTrace d) (k : Nat) (r : RunState d) :=
  let m := τ.who k
  let S := fun src => bytesOn (d := d) τ.who r.sends m src k
  let cur' := fun src => r.cur m src + min (bytesReq (τ.reads k) src) ((S src).size - r.cur m src)
  let bytes := fun src => r.buf m src ++ (S src).extract (r.cur m src) (cur' src)
  let chans := (srcsOf (τ.reads k)).map fun src => (⟨src, bytes src⟩ : (src : Member L mem) × ByteArray)
  Dag.passBytes m (τ.tick k) d cod (Dag.Timing.ofReadings m (τ.readings k) d) chans (envRestrict m (τ.inp k)) (r.cells m)

/-- Step `k` of the run: the activation; its cells, the new cursors and tails, and its
sends on the wire. -/
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

/-- How many of the connection's messages `m` has parsed from `src` before step `k`: the
frames that fit in the bytes read. -/
def pc (cod : Dag.Codecs d) (τ : PassTrace d) (k : Nat) (m src : Member L mem) : Nat :=
  (Wire.splitFrames (τ.stream cod m src k) (τ.cur cod k m src)).1

/-- The byte cursor after step `k` on the running process's connection from `src`. -/
def cur' (cod : Dag.Codecs d) (τ : PassTrace d) (k : Nat) (src : Member L mem) : Nat :=
  τ.cur cod k (τ.who k) src
    + min (bytesReq (τ.reads k) src) ((frameMsgs (τ.stream cod (τ.who k) src k)).size - τ.cur cod k (τ.who k) src)

/-- The bytes the running process hands the body for its connection from `src` at step `k`. -/
def chanBytes (cod : Dag.Codecs d) (τ : PassTrace d) (k : Nat) (src : Member L mem) : ByteArray :=
  τ.buf cod k (τ.who k) src ++ (frameMsgs (τ.stream cod (τ.who k) src k)).extract (τ.cur cod k (τ.who k) src) (τ.cur' cod k src)

def chans (cod : Dag.Codecs d) (τ : PassTrace d) (k : Nat) : List ((src : Member L mem) × ByteArray) :=
  (srcsOf (τ.reads k)).map fun src => ⟨src, τ.chanBytes cod k src⟩

/-- Activation `k`: the production body at the bytes on the running process's cells. -/
def act (cod : Dag.Codecs d) (τ : PassTrace d) (k : Nat) :=
  Dag.passBytes (τ.who k) (τ.tick k) d cod (Dag.Timing.ofReadings (τ.who k) (τ.readings k) d) (τ.chans cod k)
    (envRestrict (τ.who k) (τ.inp k)) (τ.cells cod k (τ.who k))

/-- The messages activation `k` parsed off its connection from `src`. -/
def batches (cod : Dag.Codecs d) (τ : PassTrace d) (k : Nat) : List ((src : Member L mem) × List WireMsg) :=
  (srcsOf (τ.reads k)).map fun src => ⟨src, (parseFrames (τ.chanBytes cod k src)).1⟩

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

theorem cur_eq_step (cod : Dag.Codecs d) (τ : PassTrace d) (k : Nat) (src : Member L mem) :
    (τ.run cod (k + 1)).cur (τ.who k) src = τ.cur' cod k src := by
  show (if τ.who k = τ.who k then _ else _) = _
  rw [if_pos rfl]
  unfold cur' cur
  simp only [bytesOn_eq]

theorem cur_succ_other (cod : Dag.Codecs d) (τ : PassTrace d) (k : Nat) (m src : Member L mem) (hm : τ.who k ≠ m) :
    τ.cur cod (k + 1) m src = τ.cur cod k m src := by
  show (if τ.who k = m then _ else _) = _
  rw [if_neg hm]
  rfl

theorem buf_succ_other (cod : Dag.Codecs d) (τ : PassTrace d) (k : Nat) (m src : Member L mem) (hm : τ.who k ≠ m) :
    τ.buf cod (k + 1) m src = τ.buf cod k m src := by
  show (if τ.who k = m ∧ _ then _ else _) = _
  rw [if_neg (fun h => hm h.1)]
  rfl

theorem buf_succ_self (cod : Dag.Codecs d) (τ : PassTrace d) (k : Nat) (src : Member L mem) :
    τ.buf cod (k + 1) (τ.who k) src
      = if src ∈ srcsOf (τ.reads k) then (parseFrames (τ.chanBytes cod k src)).2 else τ.buf cod k (τ.who k) src := by
  show (if τ.who k = τ.who k ∧ src ∈ srcsOf (τ.reads k) then _ else _) = _
  unfold chanBytes buf cur cur'
  simp only [true_and, bytesOn_eq]
  try rfl

theorem act_eq_wire (cod : Dag.Codecs d) (τ : PassTrace d) (k : Nat) :
    τ.act cod k
      = (let r := Dag.passWire (τ.who k) (τ.tick k) d cod (Dag.Timing.ofReadings (τ.who k) (τ.readings k) d)
          (τ.batches cod k) (envRestrict (τ.who k) (τ.inp k)) (τ.cells cod k (τ.who k))
         (r.1, r.2.1, r.2.2, (τ.chans cod k).map fun c => (parseFrames c.2).2)) := by
  unfold act Dag.passBytes chans batches
  simp only [List.map_map, Function.comp_def]

theorem act_eq_actOf (cod : Dag.Codecs d) (τ : PassTrace d) (k : Nat) : τ.act cod k = τ.actOf cod k (τ.run cod k) := by
  unfold act actOf chans chanBytes cur' cur buf cells
  simp only [bytesOn_eq]
  try rfl

theorem cells_succ_self (cod : Dag.Codecs d) (τ : PassTrace d) (k : Nat) :
    τ.cells cod (k + 1) (τ.who k) = (τ.act cod k).1 := by
  unfold cells
  simp only [run, step, dif_pos]
  rw [act_eq_actOf]

theorem cells_succ_other (cod : Dag.Codecs d) (τ : PassTrace d) (k : Nat) (m : Member L mem) (hm : τ.who k ≠ m) :
    τ.cells cod (k + 1) m = τ.cells cod k m := by
  unfold cells
  simp only [run, step, dif_neg hm]

theorem sends_eq (cod : Dag.Codecs d) (τ : PassTrace d) (k : Nat) : τ.sends cod k = (τ.act cod k).2.2.1 := by
  show (if k = k then _ else _) = _
  rw [if_pos rfl, act_eq_actOf]

/-- The items a stream output `v` of process `m` has carried along the trace by step `T`. -/
def streamOutputsAt (cod : Dag.Codecs d) (τ : PassTrace d) (T : Nat) (m : Member L mem) {ℓ : L} {α : Type} {iα : DecidableEq α} {ord : StrOrd}
    {ret : Retries} (v : Var outs (.stream ℓ α iα ord ret)) (i : Hosted m ℓ) : List α :=
  ((List.range T).filterMap fun k => if h : τ.who k = m then some (h ▸ (τ.act cod k).2.1) else none).flatMap
    fun o => o.get v i

/-- The run with member `m`'s output increments so far, in one pass — what evaluating a
trace costs (`PassTraceCheck`); `streamOutputsAt_eq_runOutputs` says it is `streamOutputsAt`. -/
def runOutputs (cod : Dag.Codecs d) (τ : PassTrace d) (m : Member L mem) :
    Nat → RunState d × List (HList (Kind.locDelta m) outs)
  | 0 => (τ.run cod 0, [])
  | k + 1 =>
    let p := τ.runOutputs cod m k
    (τ.step cod k p.1, p.2 ++ if h : τ.who k = m then [h ▸ (τ.actOf cod k p.1).2.1] else [])

theorem runOutputs_fst (cod : Dag.Codecs d) (τ : PassTrace d) (m : Member L mem) :
    ∀ k, (τ.runOutputs cod m k).1 = τ.run cod k
  | 0 => rfl
  | k + 1 => by
    show τ.step cod k (τ.runOutputs cod m k).1 = τ.step cod k (τ.run cod k)
    rw [runOutputs_fst cod τ m k]

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

theorem streamOutputsAt_eq_runOutputs (cod : Dag.Codecs d) (τ : PassTrace d) (T : Nat) (m : Member L mem) {ℓ : L} {α : Type}
    {iα : DecidableEq α} {ord : StrOrd} {ret : Retries} (v : Var outs (.stream ℓ α iα ord ret)) (i : Hosted m ℓ) :
    τ.streamOutputsAt cod T m v i = (τ.runOutputs cod m T).2.flatMap fun o => o.get v i := by
  unfold streamOutputsAt
  rw [runOutputs_snd]

/-! ### The run invariant: cursors within the stream, tails past the last frame -/

/-- The kept tail a connection should hold: the bytes past the frames that fit in the cursor. -/
def tailOf (xs : List WireMsg) (b : Nat) : ByteArray :=
  (Wire.frameAll (xs.drop (Wire.splitFrames xs b).1)).extract 0 (Wire.splitFrames xs b).2

structure RunInv (cod : Dag.Codecs d) (τ : PassTrace d) (k : Nat) : Prop where
  cur_le : ∀ m src, τ.cur cod k m src ≤ (frameMsgs (τ.stream cod m src k)).size
  buf_eq : ∀ m src, τ.buf cod k m src = tailOf (τ.stream cod m src k) (τ.cur cod k m src)

theorem stream_succ (cod : Dag.Codecs d) (τ : PassTrace d) (m src : Member L mem) (k : Nat) :
    τ.stream cod m src (k + 1) = τ.stream cod m src k ++ (if τ.who k = src then τ.sent cod k m else []) :=
  streamOf_succ τ.who (τ.sent cod) m src k

theorem size_frameMsgs_le_append (xs ys : List WireMsg) : (frameMsgs xs).size ≤ (frameMsgs (xs ++ ys)).size := by
  unfold frameMsgs
  rw [Wire.frameAll_append, ByteArray.size_append]
  exact Nat.le_add_right _ _

theorem cur'_le (cod : Dag.Codecs d) (τ : PassTrace d) (k : Nat) (src : Member L mem)
    (h : τ.cur cod k (τ.who k) src ≤ (frameMsgs (τ.stream cod (τ.who k) src k)).size) :
    τ.cur' cod k src ≤ (frameMsgs (τ.stream cod (τ.who k) src k)).size := by
  unfold cur'
  omega

theorem cur_le_cur' (cod : Dag.Codecs d) (τ : PassTrace d) (k : Nat) (src : Member L mem) :
    τ.cur cod k (τ.who k) src ≤ τ.cur' cod k src := Nat.le_add_right _ _

theorem cur'_of_not_mem (cod : Dag.Codecs d) (τ : PassTrace d) (k : Nat) (src : Member L mem)
    (h : src ∉ srcsOf (τ.reads k)) : τ.cur' cod k src = τ.cur cod k (τ.who k) src := by
  unfold cur'
  rw [bytesReq_eq_zero h, Nat.zero_min, Nat.add_zero]

/-- The bytes handed the body on a connection: the stream past the parsed frames, up to the
new cursor. -/
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

/-- What activation `k` parses off its connection from `src`: the next messages of the
stream, as many as the bytes read complete. -/
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

theorem runInv_zero (cod : Dag.Codecs d) (τ : PassTrace d) : τ.RunInv cod 0 :=
  { cur_le := fun _ _ => Nat.zero_le _
    buf_eq := fun m src => by
      show ByteArray.empty = tailOf (streamOf τ.who (τ.sent cod) m src 0) 0
      unfold tailOf streamOf activationsOf
      simp [Wire.splitFrames, Wire.frameAll] }

theorem tailOf_append_of_le (xs ys : List WireMsg) (b : Nat) (hb : b ≤ (Wire.frameAll xs).size) :
    tailOf (xs ++ ys) b = tailOf xs b := Wire.tail_append_of_le xs ys b hb

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

theorem runInv (cod : Dag.Codecs d) (τ : PassTrace d) : ∀ k, τ.RunInv cod k
  | 0 => τ.runInv_zero cod
  | k + 1 => τ.runInv_succ cod k (τ.runInv cod k)

/-! ### The parsed counts: idle-stable, within the stream, monotone, before this step's sends -/

theorem pc_le (cod : Dag.Codecs d) (τ : PassTrace d) (k : Nat) (m src : Member L mem) :
    τ.pc cod k m src ≤ (τ.stream cod m src k).length := Wire.splitFrames_fst_le_length _ _

theorem pc_succ_other (cod : Dag.Codecs d) (τ : PassTrace d) (k : Nat) (m src : Member L mem) (hm : τ.who k ≠ m) :
    τ.pc cod (k + 1) m src = τ.pc cod k m src := by
  unfold pc
  rw [cur_succ_other cod τ k m src hm, stream_succ, Wire.splitFrames_append_of_le _ _ _ ((τ.runInv cod k).cur_le m src)]

/-- The count after the running process's step, against the stream before its own sends. -/
theorem pc_succ_self (cod : Dag.Codecs d) (τ : PassTrace d) (k : Nat) (src : Member L mem) :
    τ.pc cod (k + 1) (τ.who k) src = (Wire.splitFrames (τ.stream cod (τ.who k) src k) (τ.cur' cod k src)).1 := by
  unfold pc
  rw [show τ.cur cod (k + 1) (τ.who k) src = τ.cur' cod k src from cur_eq_step cod τ k src, stream_succ,
    Wire.splitFrames_append_of_le _ _ _ (τ.cur'_le cod k src ((τ.runInv cod k).cur_le _ _))]

theorem pc_mono (cod : Dag.Codecs d) (τ : PassTrace d) (k : Nat) (m src : Member L mem) :
    τ.pc cod k m src ≤ τ.pc cod (k + 1) m src := by
  by_cases hm : τ.who k = m
  · subst hm
    rw [pc_succ_self]
    exact Wire.splitFrames_fst_mono _ (τ.cur_le_cur' cod k src)
  · rw [pc_succ_other cod τ k m src hm]

theorem pc_succ_self_le (cod : Dag.Codecs d) (τ : PassTrace d) (k : Nat) (src : Member L mem) :
    τ.pc cod (k + 1) (τ.who k) src ≤ (τ.stream cod (τ.who k) src k).length := by
  rw [pc_succ_self]
  exact Wire.splitFrames_fst_le_length _ _

theorem pc_succ_self_of_not_mem (cod : Dag.Codecs d) (τ : PassTrace d) (k : Nat) (src : Member L mem)
    (h : src ∉ srcsOf (τ.reads k)) : τ.pc cod (k + 1) (τ.who k) src = τ.pc cod k (τ.who k) src := by
  rw [pc_succ_self, cur'_of_not_mem cod τ k src h]
  rfl

/-- **The batches activation `k` parses**: per connection read, the next messages of its
stream up to the new parsed count. -/
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

Both are read off the trace: `who`, what each activation put on each connection
(`sent`), and the parsed counts (`pc k m src`: how many of `src`'s messages to `m` the
receiver has parsed before step `k`). At a network node, the cursor of receiver `i` from
sender `j` at step `t` is the number of that node's messages among the parsed prefix at
`t + 1` (the activation at `t`, if it is `i`'s, included); at a timer node the script is
the readings at the member's `n`-th activation. The invariant `TraceQueues` says what the
lock-step cells of member `m` hold at step `k` relative to those counts. The facts about
`pc` the proofs need are the four in `PcOk`; the trace's counts satisfy them
(`PassTrace.pcOk`). -/

section Induced

variable (who : Nat → Member L mem) (sent : Nat → Member L mem → List WireMsg) (pc : Nat → Member L mem → Member L mem → Nat)
  (tick : Nat → Bool)

/-- What the induction needs of the parsed counts. -/
structure PcOk : Prop where
  idle : ∀ k m src, who k ≠ m → pc (k + 1) m src = pc k m src
  le : ∀ k m src, pc k m src ≤ (streamOf who sent m src k).length
  mono : ∀ k m src, pc k m src ≤ pc (k + 1) m src
  self_le : ∀ k src, pc (k + 1) (who k) src ≤ (streamOf who sent (who k) src k).length

/-- The messages of a connection `m` has not yet parsed before step `k`, seen by a node
(`f`: the node's and sender's messages, decoded). -/
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

theorem decodeAt_eq_filterMap {α : Type} (cod : Codec α) (msgs : List WireMsg) (base j : Nat) :
    decodeAt cod msgs base j = msgs.filterMap (bcSel cod base j) := rfl

theorem decodeAt_filterMap_eq {α : Type} {p : L} (cod : Codec (Nat × α)) (msgs : List WireMsg) (base : Nat)
    (addr : Fin (mem p) → Nat) (i : Fin (mem p)) (j : Nat) :
    (decodeAt cod msgs base j).filterMap (fun dx => if dx.1 = addr i then some dx.2 else none)
      = msgs.filterMap (dxSel cod base addr i j) := by
  rw [decodeAt_eq_filterMap, List.filterMap_filterMap]
  rfl

mutual

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

noncomputable def Dag.traceSched (readings : (k : Nat) → TickReadings (who k)) :
    {Γ : Ctx L mem} → {outs : List (Kind L mem)} → (d : Dag Γ outs) → Dag.Codecs d → Nat → Dag.Sched d
  | _, _, .ret _, _, _ => ()
  | _, _, .letN n rest, cod, base =>
    (Node.traceSched readings n cod.1 base, Dag.traceSched readings rest cod.2 (Node.wireReceivers n base).2)

end

mutual

/-- **The queue invariant** at step `k` for member `m`'s cells: each receiving end holds the
connection's unparsed messages and the parsed count; each timer cell the activation count. -/
def Node.TraceQueues (k : Nat) (m : Member L mem) :
    {Γ : Ctx L mem} → {κs : List (Kind L mem)} → (n : Node Γ κs) → Node.Codecs n → Nat → Node.LocState m n → Prop
  | _, _, .broadcast_closed (c := c) _ _, cod, base, s => ∀ i j,
    s i j = ⟨unconsumed who sent pc (bcSel cod base j.val) m ⟨c, j⟩ k, ccount who sent pc (bcSel cod base j.val) m ⟨c, j⟩ k⟩
  | _, _, .demux (c := c) _ _ addr, cod, base, s => ∀ i j,
    s i j = ⟨unconsumed who sent pc (dxSel cod base addr i.1 j.val) m ⟨c, j⟩ k,
      ccount who sent pc (dxSel cod base addr i.1 j.val) m ⟨c, j⟩ k⟩
  | _, _, .source_interval .., _, _, s => ∀ i, s i = (ticksOf who tick m k).length
  | _, _, .time .., _, _, s => ∀ i, s i = (ticksOf who tick m k).length
  | _, _, .call N _, cod, base, s => Dag.TraceQueues k m N cod base s
  | _, _, .fixN _ body, cod, base, s => Dag.TraceQueues k m body cod base s.1
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

def Dag.TraceQueues (k : Nat) (m : Member L mem) :
    {Γ : Ctx L mem} → {outs : List (Kind L mem)} → (d : Dag Γ outs) → Dag.Codecs d → Nat → Dag.LocState m d → Prop
  | _, _, .ret _, _, _, _ => True
  | _, _, .letN n rest, cod, base, s =>
    Node.TraceQueues k m n cod.1 base s.1 ∧ Dag.TraceQueues k m rest cod.2 (Node.wireReceivers n base).2 s.2

end

/-! ### Facts about one connection -/


/-- The batches' messages for one connection: the batch of that sender, if it was read
(a sender not read parses nothing). -/
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


theorem cumMax_eq_of_mono (c : Nat → Nat) (h : ∀ t, c t ≤ c (t + 1)) : ∀ t, cumMax c t = c t
  | 0 => rfl
  | t + 1 => by
    show max (cumMax c t) (c (t + 1)) = c (t + 1)
    rw [cumMax_eq_of_mono c h t]
    exact Nat.max_eq_right (h t)

variable (hpc : PcOk who sent pc)
include hpc

theorem ccount_mono {α : Type} (f : WireMsg → Option α) (m src : Member L mem) (k : Nat) :
    ccount who sent pc f m src k ≤ ccount who sent pc f m src (k + 1) := by
  unfold ccount
  rw [streamOf_succ, show pc (k + 1) m src = pc k m src + (pc (k + 1) m src - pc k m src) by
    have := hpc.mono k m src; omega,
    List.take_add, List.take_append_of_le_length (hpc.le k m src), List.filterMap_append, List.length_append]
  exact Nat.le_add_right _ _

theorem cursor_eq {α : Type} (f : WireMsg → Option α) (m src : Member L mem) (k : Nat) :
    cumMax (fun t => ccount who sent pc f m src (t + 1)) k = ccount who sent pc f m src (k + 1) :=
  cumMax_eq_of_mono _ (fun t => ccount_mono who sent pc hpc f m src (t + 1)) k

/-- A receiving end's cell after the sender's activation `k` reaches it (the receiver idle). -/
theorem chan_idle_sender {α : Type} (f : WireMsg → Option α) (m src : Member L mem) (k : Nat) (hm : who k ≠ m)
    (hsrc : who k = src) (cell : ChanCell α)
    (hcell : cell = ⟨unconsumed who sent pc f m src k, ccount who sent pc f m src k⟩) :
    cell.enqueue ((sent k m).filterMap f) = ⟨unconsumed who sent pc f m src (k + 1), ccount who sent pc f m src (k + 1)⟩ := by
  rw [hcell]
  unfold unconsumed ccount
  rw [hpc.idle k m src hm, streamOf_succ, if_pos hsrc, List.drop_append_of_le_length (hpc.le k m src),
    List.take_append_of_le_length (hpc.le k m src), List.filterMap_append]
  rfl

/-- … and after another sender's activation (nothing arrives). -/
theorem chan_idle_other {α : Type} (f : WireMsg → Option α) (m src : Member L mem) (k : Nat) (hm : who k ≠ m)
    (hsrc : who k ≠ src) (cell : ChanCell α)
    (hcell : cell = ⟨unconsumed who sent pc f m src k, ccount who sent pc f m src k⟩) :
    cell = ⟨unconsumed who sent pc f m src (k + 1), ccount who sent pc f m src (k + 1)⟩ := by
  rw [hcell]
  unfold unconsumed ccount
  rw [hpc.idle k m src hm, streamOf_succ, if_neg hsrc, List.append_nil]

/-- The unparsed messages before the receiver's activation: what it parses now, then the rest. -/
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

theorem ccount_active {α : Type} (f : WireMsg → Option α) (src : Member L mem) (k : Nat) :
    ccount who sent pc f (who k) src (k + 1) = ccount who sent pc f (who k) src k + (newMsgs who sent pc f src k).length := by
  unfold ccount newMsgs
  rw [streamOf_succ, List.take_append_of_le_length (hpc.self_le k src),
    show pc (k + 1) (who k) src = pc k (who k) src + (pc (k + 1) (who k) src - pc k (who k) src) by
      have := hpc.mono k (who k) src; omega,
    List.take_add, List.filterMap_append, List.length_append]
  simp only [Nat.add_sub_cancel_left]

/-- A receiving end's cell through the receiver's own activation `k`: the newly parsed
messages are delivered (the cursor is their count). -/
theorem chan_active_step {α : Type} (f : WireMsg → Option α) (src : Member L mem) (k : Nat) (cell : ChanCell α)
    (hcell : cell = ⟨unconsumed who sent pc f (who k) src k, ccount who sent pc f (who k) src k⟩) :
    (cell.step (ccount who sent pc f (who k) src (k + 1)) []).1
      = ⟨((streamOf who sent (who k) src k).drop (pc (k + 1) (who k) src)).filterMap f,
         ccount who sent pc f (who k) src (k + 1)⟩ := by
  rw [hcell, unconsumed_split who sent pc hpc f src k, ccount_active who sent pc hpc f src k]
  unfold ChanCell.step
  simp only [Nat.add_sub_cancel_left, List.take_left, List.drop_left, List.append_nil]

/-- … then its own sends are enqueued, if it is the sender. -/
theorem chan_active_sender {α : Type} (f : WireMsg → Option α) (src : Member L mem) (k : Nat) (hsrc : who k = src)
    (cell : ChanCell α)
    (hcell : cell = ⟨unconsumed who sent pc f (who k) src k, ccount who sent pc f (who k) src k⟩) :
    (cell.step (ccount who sent pc f (who k) src (k + 1)) []).1.enqueue ((sent k (who k)).filterMap f)
      = ⟨unconsumed who sent pc f (who k) src (k + 1), ccount who sent pc f (who k) src (k + 1)⟩ := by
  rw [chan_active_step who sent pc hpc f src k cell hcell]
  unfold unconsumed
  rw [streamOf_succ, if_pos hsrc, List.drop_append_of_le_length (hpc.self_le k src), List.filterMap_append]
  rfl

theorem chan_active_other {α : Type} (f : WireMsg → Option α) (src : Member L mem) (k : Nat) (hsrc : who k ≠ src)
    (cell : ChanCell α)
    (hcell : cell = ⟨unconsumed who sent pc f (who k) src k, ccount who sent pc f (who k) src k⟩) :
    (cell.step (ccount who sent pc f (who k) src (k + 1)) []).1
      = ⟨unconsumed who sent pc f (who k) src (k + 1), ccount who sent pc f (who k) src (k + 1)⟩ := by
  rw [chan_active_step who sent pc hpc f src k cell hcell]
  unfold unconsumed
  rw [streamOf_succ, if_neg hsrc, List.append_nil]

/-- The newly parsed messages are what the cursor delivers. -/
theorem chan_arrivals {α : Type} (f : WireMsg → Option α) (src : Member L mem) (k : Nat) (cell : ChanCell α)
    (hcell : cell = ⟨unconsumed who sent pc f (who k) src k, ccount who sent pc f (who k) src k⟩) :
    cell.pending.take (ccount who sent pc f (who k) src (k + 1) - cell.delivered) = newMsgs who sent pc f src k := by
  rw [hcell, unconsumed_split who sent pc hpc f src k, ccount_active who sent pc hpc f src k]
  simp only [Nat.add_sub_cancel_left, List.take_left]

/-! ### The invariant along the lock-step run -/

omit hpc in
mutual

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

theorem Dag.traceQueues_init (m : Member L mem) : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → (d : Dag Γ outs) →
    (cod : Dag.Codecs d) → (base : Nat) →
    Dag.TraceQueues who sent pc tick 0 m d cod base (Dag.restrict m d (Dag.init d))
  | _, _, .ret _, _, _ => trivial
  | _, _, .letN n rest, cod, base =>
    ⟨Node.traceQueues_init m n cod.1 base, Dag.traceQueues_init m rest cod.2 _⟩

end

mutual

/-- **Idle step**: a process that is not running receives the active one's bytes. -/
theorem Node.traceQueues_idle (k : Nat) (m : Member L mem) (hm : who k ≠ m) :
    {Γ : Ctx L mem} → {κs : List (Kind L mem)} → (n : Node Γ κs) → (cod : Node.Codecs n) → (base : Nat) →
    (s : Node.LocState m n) → Node.TraceQueues who sent pc tick k m n cod base s →
    Node.TraceQueues who sent pc tick (k + 1) m n cod base
      (Node.enqueueInbox m n (Node.inboxOfWire m n cod base [⟨who k, sent k m⟩]).1 s)
  | _, _, .broadcast_closed (c := c) _ _, cod, base, s, h => fun i j => by
    show (s i j).enqueue ((if (⟨c, j⟩ : Member L mem) = who k then decodeAt cod (sent k m) base j.val else []) ++ []) = _
    rw [List.append_nil]
    by_cases hs : (⟨c, j⟩ : Member L mem) = who k
    · rw [if_pos hs]
      exact chan_idle_sender who sent pc hpc (bcSel cod base j.val) m ⟨c, j⟩ k hm hs.symm (s i j) (h i j)
    · rw [if_neg hs, ChanCell.enqueue_nil]
      exact chan_idle_other who sent pc hpc (bcSel cod base j.val) m ⟨c, j⟩ k hm (Ne.symm hs) (s i j) (h i j)
  | _, _, .demux (c := c) _ _ addr, cod, base, s, h => fun i j => by
    show (s i j).enqueue ((if (⟨c, j⟩ : Member L mem) = who k then
        (decodeAt cod (sent k m) base j.val).filterMap (fun dx => if dx.1 = addr i.1 then some dx.2 else none)
      else []) ++ []) = _
    rw [List.append_nil]
    by_cases hs : (⟨c, j⟩ : Member L mem) = who k
    · rw [if_pos hs, show (decodeAt cod (sent k m) base j.val).filterMap (fun dx => if dx.1 = addr i.1 then some dx.2 else none)
          = (sent k m).filterMap (dxSel cod base addr i.1 j.val) from decodeAt_filterMap_eq _ _ _ _ _ _]
      exact chan_idle_sender who sent pc hpc (dxSel cod base addr i.1 j.val) m ⟨c, j⟩ k hm hs.symm (s i j) (h i j)
    · rw [if_neg hs, ChanCell.enqueue_nil]
      exact chan_idle_other who sent pc hpc (dxSel cod base addr i.1 j.val) m ⟨c, j⟩ k hm (Ne.symm hs) (s i j) (h i j)
  | _, _, .source_interval .., _, _, s, h => fun i => by
    show s i = _
    rw [h i, ticksOf_succ, if_neg (fun hh => hm hh.1), List.append_nil]
  | _, _, .time .., _, _, s, h => fun i => by
    show s i = _
    rw [h i, ticksOf_succ, if_neg (fun hh => hm hh.1), List.append_nil]
  | _, _, .call N _, cod, base, s, h => Dag.traceQueues_idle k m hm N cod base s h
  | _, _, .fixN _ body, cod, base, s, h => Dag.traceQueues_idle k m hm body cod base s.1 h
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

/-- The cursor the induced schedule hands a receiving end at step `k`: the parsed count
through `k`. -/
theorem traceSched_cursor {α : Type} (f : WireMsg → Option α) {c p : L} (k : Nat) (i : Hosted (who k) p) (j : Fin (mem c)) :
    cumMax (fun t => ccount who sent pc f ⟨p, i.1⟩ ⟨c, j⟩ (t + 1)) k = ccount who sent pc f (who k) ⟨c, j⟩ (k + 1) := by
  rw [cursor_eq who sent pc hpc, i.2]

mutual

/-- **Active step**: the running process delivers the newly parsed messages (its cursors),
ticks, and enqueues what it sent itself. -/
theorem Node.traceQueues_active (readings : (k : Nat) → TickReadings (who k)) (k : Nat) :
    {Γ : Ctx L mem} → {κs : List (Kind L mem)} → (n : Node Γ κs) → (cod : Node.Codecs n) → (base : Nat) →
    (Δ : HList (Kind.locDelta (who k)) Γ) → (s : Node.LocState (who k) n) →
    Node.TraceQueues who sent pc tick k (who k) n cod base s →
    Node.TraceQueues who sent pc tick (k + 1) (who k) n cod base
      (Node.enqueueInbox (who k) n (Node.inboxOfWire (who k) n cod base [⟨who k, sent k (who k)⟩]).1
        (Node.locStep (who k) (tick k) n (Node.cursorsAt (who k) n (Node.traceSched who sent pc tick readings n cod base) k s) Δ s).1)
  | _, _, .broadcast_closed (c := c) _ _, cod, base, Δ, s, h => fun i j => by
    show (((s i j).step (cumMax (fun t => ccount who sent pc (bcSel cod base j.val) ⟨_, i.1⟩ ⟨c, j⟩ (t + 1)) k) []).1).enqueue
        ((if (⟨c, j⟩ : Member L mem) = who k then decodeAt cod (sent k (who k)) base j.val else []) ++ []) = _
    rw [List.append_nil, traceSched_cursor who sent pc hpc]
    by_cases hs : (⟨c, j⟩ : Member L mem) = who k
    · rw [if_pos hs]
      exact chan_active_sender who sent pc hpc (bcSel cod base j.val) ⟨c, j⟩ k hs.symm (s i j) (h i j)
    · rw [if_neg hs, ChanCell.enqueue_nil]
      exact chan_active_other who sent pc hpc (bcSel cod base j.val) ⟨c, j⟩ k (Ne.symm hs) (s i j) (h i j)
  | _, _, .demux (c := c) _ _ addr, cod, base, Δ, s, h => fun i j => by
    show (((s i j).step (cumMax (fun t => ccount who sent pc (dxSel cod base addr i.1 j.val) ⟨_, i.1⟩ ⟨c, j⟩ (t + 1)) k) []).1).enqueue
        ((if (⟨c, j⟩ : Member L mem) = who k then
          (decodeAt cod (sent k (who k)) base j.val).filterMap (fun dx => if dx.1 = addr i.1 then some dx.2 else none)
        else []) ++ []) = _
    rw [List.append_nil, traceSched_cursor who sent pc hpc]
    by_cases hs : (⟨c, j⟩ : Member L mem) = who k
    · rw [if_pos hs, show (decodeAt cod (sent k (who k)) base j.val).filterMap (fun dx => if dx.1 = addr i.1 then some dx.2 else none)
          = (sent k (who k)).filterMap (dxSel cod base addr i.1 j.val) from decodeAt_filterMap_eq _ _ _ _ _ _]
      exact chan_active_sender who sent pc hpc (dxSel cod base addr i.1 j.val) ⟨c, j⟩ k hs.symm (s i j) (h i j)
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
  | _, _, .fixN _ body, cod, base, _, s, h => Dag.traceQueues_active readings k body cod base _ s.1 h
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

/-- **Arrivals**: at its activation `k`, what the induced cursors deliver to the running
process is the decode of the batches it parsed. -/
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
    show (s i j).pending.take (cumMax (fun t => ccount who sent pc (bcSel cod base j.val) ⟨_, i.1⟩ ⟨c, j⟩ (t + 1)) k
        - (s i j).delivered) = _
    rw [traceSched_cursor who sent pc hpc, chan_arrivals who sent pc hpc _ _ k (s i j) (h i j)]
    show _ = (srcs.map _).flatMap fun (b : (src : Member L mem) × List WireMsg) =>
      if (⟨c, j⟩ : Member L mem) = b.1 then decodeAt cod b.2 base j.val else []
    rw [flatMap_batches srcs hnd _ (fun l => decodeAt cod l base j.val) rfl ⟨c, j⟩
        (fun hs => by rw [hout _ hs, Nat.sub_self, List.take_zero])]
    rfl
  | _, _, .demux (c := c) _ _ addr, cod, base, s, h => by
    funext i j
    show (s i j).pending.take (cumMax (fun t => ccount who sent pc (dxSel cod base addr i.1 j.val) ⟨_, i.1⟩ ⟨c, j⟩ (t + 1)) k
        - (s i j).delivered) = _
    rw [traceSched_cursor who sent pc hpc, chan_arrivals who sent pc hpc _ _ k (s i j) (h i j)]
    show _ = (srcs.map _).flatMap fun (b : (src : Member L mem) × List WireMsg) => if (⟨c, j⟩ : Member L mem) = b.1 then
      (decodeAt cod b.2 base j.val).filterMap (fun dx => if dx.1 = addr i.1 then some dx.2 else none) else []
    rw [flatMap_batches srcs hnd _
        (fun l => (decodeAt cod l base j.val).filterMap (fun dx => if dx.1 = addr i.1 then some dx.2 else none)) rfl ⟨c, j⟩
        (fun hs => by rw [hout _ hs, Nat.sub_self, List.take_zero])]
    exact (decodeAt_filterMap_eq _ _ _ _ _ _).symm
  | _, _, .call N _, cod, base, s, h => Dag.traceQueues_arrivals readings k srcs hnd hout N cod base s h
  | _, _, .fixN _ body, cod, base, s, h => Dag.traceQueues_arrivals readings k srcs hnd hout body cod base s.1 h
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

/-- **Timing**: at its activation `k`, the induced scripts read at the process's tick counts
are the readings it took. -/
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
  | _, _, .fixN _ body, cod, base, s, h => Dag.traceQueues_timing readings k htk body cod base s.1 h
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

/-- The trace's parsed counts satisfy what the induction needs. -/
theorem pcOk (cod : Dag.Codecs d) (τ : PassTrace d) : PcOk τ.who (τ.sent cod) (τ.pc cod) :=
  { idle := fun k m src hm => τ.pc_succ_other cod k m src hm
    le := fun k m src => τ.pc_le cod k m src
    mono := fun k m src => τ.pc_mono cod k m src
    self_le := fun k src => τ.pc_succ_self_le cod k src }

/-- The schedule the trace induces: per network node, per receiver and sender, the cursor
at step `t` is the number of that connection's messages the receiver has parsed through
`t`; per timer node, the script is the readings indexed by activation. -/
noncomputable def sched (cod : Dag.Codecs d) (τ : PassTrace d) : Dag.Sched d :=
  Dag.traceSched τ.who (τ.sent cod) (τ.pc cod) τ.tick τ.readings d cod 0

/-- The interleaving: the trace's processes and inputs, every activation a tick, the
induced schedule. -/
noncomputable def interleaving (cod : Dag.Codecs d) (τ : PassTrace d) : Interleaving d :=
  ⟨τ.who, τ.tick, τ.inp, τ.sched cod⟩

/-- **The invariant the induction carries** at step `k`: the lock-step queues are the
trace's (`TraceQueues`), the process cells are the model's loop cells, and every earlier
activation's bytes are the model's. -/
structure Inv (cod : Dag.Codecs d) (τ : PassTrace d) (k : Nat) : Prop where
  queues : ∀ m, Dag.TraceQueues τ.who (τ.sent cod) (τ.pc cod) τ.tick k m d cod 0 ((τ.interleaving cod).view k m)
  cells : ∀ m, τ.cells cod k m = (τ.interleaving cod).loopView k m
  sends : ∀ k' < k, τ.sends cod k' = (τ.interleaving cod).wireOut cod k'

/-- Under the invariant at `k`, activation `k` at the wire is the model's loop body on the
batches the trace parsed. -/
theorem pass_eq_of_inv (cod : Dag.Codecs d) (τ : PassTrace d) (k : Nat) (h : τ.Inv cod k) :
    Dag.pass (τ.who k) (τ.tick k) d (Dag.Timing.ofReadings (τ.who k) (τ.readings k) d)
        (Dag.inboxOfWire (τ.who k) d cod 0 (τ.batches cod k)).1 (envRestrict (τ.who k) (τ.inp k)) (τ.cells cod k (τ.who k))
      = (τ.interleaving cod).loopPass k := by
  unfold Interleaving.loopPass
  rw [h.cells]
  -- the arrivals: the induced cursors deliver the parsed batches
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

theorem inv_zero (cod : Dag.Codecs d) (τ : PassTrace d) : τ.Inv cod 0 :=
  { queues := fun m => Dag.traceQueues_init τ.who (τ.sent cod) (τ.pc cod) τ.tick m d cod 0
    cells := fun _ => rfl
    sends := fun _ h => absurd h (Nat.not_lt_zero _) }

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

theorem inv (cod : Dag.Codecs d) (τ : PassTrace d) : ∀ k, τ.Inv cod k
  | 0 => τ.inv_zero cod
  | k + 1 => τ.inv_succ cod k (τ.inv cod k)

/-- **The trace's cells are the model's loop cells.** -/
theorem cells_eq (cod : Dag.Codecs d) (τ : PassTrace d) (k : Nat) (m : Member L mem) :
    τ.cells cod k m = (τ.interleaving cod).loopView k m := (τ.inv cod k).cells m

/-- **The induced interleaving is batched by the parsed frames**: its arrivals at step `k`
decode from the batches activation `k` parsed. -/
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

noncomputable def batched (cod : Dag.Codecs d) (τ : PassTrace d) : (τ.interleaving cod).Batched cod :=
  ⟨τ.batches cod, τ.arrivals_eq cod⟩

/-- The model's timing at a tick step `k` is the readings the trace took. -/
theorem timing_eq (cod : Dag.Codecs d) (τ : PassTrace d) (k : Nat) (htk : τ.tick k = true) :
    (τ.interleaving cod).timing k = Dag.Timing.ofReadings (τ.who k) (τ.readings k) d := by
  show Dag.timingAt (τ.who k) d (Dag.traceSched τ.who (τ.sent cod) (τ.pc cod) τ.tick τ.readings d cod 0)
    (Dag.arrivedOnly (τ.who k) d ((τ.interleaving cod).view k (τ.who k))) = _
  exact Dag.traceQueues_timing τ.who (τ.sent cod) (τ.pc cod) τ.tick τ.readings k htk d cod 0 _ ((τ.inv cod k).queues (τ.who k))

/-- **The trace's run is the model's run at the wire**: every activation's cells, outputs
and bytes are the batched interleaving's loop body at the wire. -/
theorem act_eq (cod : Dag.Codecs d) (τ : PassTrace d) (k : Nat) :
    ((τ.act cod k).1, (τ.act cod k).2.1, (τ.act cod k).2.2.1) = (τ.batched cod).loopPassWire k := by
  rw [act_eq_wire, (τ.batched cod).loop_pass_wire k]
  show (let r := Dag.pass (τ.who k) (τ.tick k) d _ (Dag.inboxOfWire (τ.who k) d cod 0 (τ.batches cod k)).1 _ _
    (r.1, r.2.1, (Dag.wireEncode (τ.who k) d cod 0 r.2.2).1)) = _
  rw [τ.pass_eq_of_inv cod k (τ.inv cod k), (τ.interleaving cod).loop_pass k]
  rfl

/-- **What a process outputs along the trace is what it outputs in the model.** -/
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

/-! ## The trace (D90, D93): what a deployment produces

**A trace at the bytes** is four functions — which process ran at each activation, the
inputs it was given, the clock and timer readings it took, and how many bytes it read
off each of its connections. Nothing else: a connection is a byte queue (TCP), a process
reads some prefix of what is queued — cut anywhere, mid-frame included — and runs the
production body `Dag.iterateBytes` on it (a tick, then stutters until its knots are
quiet, D93); the complete frames are its messages, the incomplete tail is kept, its
sends go on its connections as frames. There is no assumption to state: every
`who`/`inp`/`readings`/`reads` is a trace. The trace does not see the stutters: they are
the body's. `WireTrace.expand` is the pass-level picture — every pass a step — and
`WireTrace.streamOutputsAt_eq` says the trace's outputs are the model's at the
expanded step; `paxos_safe_wired` (`Paxos/WiredSafety.lean`) quantifies over traces. -/

structure WireTrace {Γ : Ctx L mem} {outs : List (Kind L mem)} (d : Dag Γ outs) where
  who : Nat → Member L mem
  inp : Nat → HList (Kind.delta (mem := mem)) Γ
  readings : (k : Nat) → TickReadings (who k)
  reads : Nat → List (Member L mem × Nat)

namespace WireTrace

variable {Γ : Ctx L mem} {outs : List (Kind L mem)} {d : Dag Γ outs}

/-- Activation `k` on a run state: the production body at the bytes on the running
process's cells, handed each listed connection's kept tail plus the requested bytes (as
many as are there). -/
def actOf (cod : Dag.Codecs d) (τ : WireTrace d) (k : Nat) (r : RunState d) :=
  let m := τ.who k
  let S := fun src => PassTrace.bytesOn (d := d) τ.who r.sends m src k
  let cur' := fun src => r.cur m src + min (bytesReq (τ.reads k) src) ((S src).size - r.cur m src)
  let bytes := fun src => r.buf m src ++ (S src).extract (r.cur m src) (cur' src)
  let chans := (srcsOf (τ.reads k)).map fun src => (⟨src, bytes src⟩ : (src : Member L mem) × ByteArray)
  Dag.iterateBytes m true d cod (Dag.Timing.ofReadings m (τ.readings k) d) chans (envRestrict m (τ.inp k)) (r.cells m)

/-- Step `k` of the run: the activation; its cells, the new cursors and tails, and its
sends on the wire. -/
def step (cod : Dag.Codecs d) (τ : WireTrace d) (k : Nat) (r : RunState d) : RunState d :=
  let m := τ.who k
  let S := fun src => PassTrace.bytesOn (d := d) τ.who r.sends m src k
  let cur' := fun src => r.cur m src + min (bytesReq (τ.reads k) src) ((S src).size - r.cur m src)
  let bytes := fun src => r.buf m src ++ (S src).extract (r.cur m src) (cur' src)
  let a := τ.actOf cod k r
  { cells := fun m' => if h : m = m' then h ▸ a.1 else r.cells m'
    cur := fun m' src => if m = m' then cur' src else r.cur m' src
    buf := fun m' src => if m = m' ∧ src ∈ srcsOf (τ.reads k) then (parseFrames (bytes src)).2 else r.buf m' src
    sends := fun k' => if k' = k then a.2.2.1 else r.sends k' }

def run (cod : Dag.Codecs d) (τ : WireTrace d) : Nat → RunState d
  | 0 => ⟨fun m => PassTrace.initCells m, fun _ _ => 0, fun _ _ => .empty, fun _ => []⟩
  | k + 1 => τ.step cod k (τ.run cod k)

/-- Process `m`'s cells before activation `k`. -/
def cells (cod : Dag.Codecs d) (τ : WireTrace d) (k : Nat) (m : Member L mem) : Dag.LocState m d := (τ.run cod k).cells m

/-- The messages activation `k` put on the wire. -/
def sends (cod : Dag.Codecs d) (τ : WireTrace d) (k : Nat) : List WireMsg := (τ.run cod (k + 1)).sends k

/-- Activation `k`: the production body at the bytes on the running process's cells. -/
def act (cod : Dag.Codecs d) (τ : WireTrace d) (k : Nat) := τ.actOf cod k (τ.run cod k)

/-- The items a stream output `v` of process `m` has carried along the trace by activation `T`. -/
def streamOutputsAt (cod : Dag.Codecs d) (τ : WireTrace d) (T : Nat) (m : Member L mem) {ℓ : L} {α : Type} {iα : DecidableEq α} {ord : StrOrd}
    {ret : Retries} (v : Var outs (.stream ℓ α iα ord ret)) (i : Hosted m ℓ) : List α :=
  ((List.range T).filterMap fun k => if h : τ.who k = m then some (h ▸ (τ.act cod k).2.1) else none).flatMap
    fun o => o.get v i

/-- The run with member `m`'s output increments so far, in one pass — what evaluating a
trace costs (`WireTraceCheck`); `streamOutputsAt_eq_runOutputs` says it is `streamOutputsAt`. -/
def runOutputs (cod : Dag.Codecs d) (τ : WireTrace d) (m : Member L mem) :
    Nat → RunState d × List (HList (Kind.locDelta m) outs)
  | 0 => (τ.run cod 0, [])
  | k + 1 =>
    let p := τ.runOutputs cod m k
    (τ.step cod k p.1, p.2 ++ if h : τ.who k = m then [h ▸ (τ.actOf cod k p.1).2.1] else [])

theorem runOutputs_fst (cod : Dag.Codecs d) (τ : WireTrace d) (m : Member L mem) :
    ∀ k, (τ.runOutputs cod m k).1 = τ.run cod k
  | 0 => rfl
  | k + 1 => by
    show τ.step cod k (τ.runOutputs cod m k).1 = τ.step cod k (τ.run cod k)
    rw [runOutputs_fst cod τ m k]

theorem runOutputs_snd (cod : Dag.Codecs d) (τ : WireTrace d) (m : Member L mem) :
    ∀ k, (τ.runOutputs cod m k).2
      = (List.range k).filterMap fun k => if h : τ.who k = m then some (h ▸ (τ.act cod k).2.1) else none
  | 0 => rfl
  | k + 1 => by
    show (τ.runOutputs cod m k).2 ++ (if h : τ.who k = m then [h ▸ (τ.actOf cod k (τ.runOutputs cod m k).1).2.1] else []) = _
    rw [List.range_succ, List.filterMap_append, runOutputs_snd cod τ m k, runOutputs_fst]
    congr 1
    by_cases h : τ.who k = m
    · simp [h, act]
    · simp [h]

theorem streamOutputsAt_eq_runOutputs (cod : Dag.Codecs d) (τ : WireTrace d) (T : Nat) (m : Member L mem) {ℓ : L} {α : Type}
    {iα : DecidableEq α} {ord : StrOrd} {ret : Retries} (v : Var outs (.stream ℓ α iα ord ret)) (i : Hosted m ℓ) :
    τ.streamOutputsAt cod T m v i = (τ.runOutputs cod m T).2.flatMap fun o => o.get v i := by
  unfold streamOutputsAt
  rw [runOutputs_snd]

/-! ### The expansion: every pass a step -/

/-- The body of activation `k` as `Dag.iterate`: on the parsed frames of the bytes it read. -/
def body (cod : Dag.Codecs d) (τ : WireTrace d) (k : Nat) :=
  let r := τ.run cod k
  let m := τ.who k
  let S := fun src => PassTrace.bytesOn (d := d) τ.who r.sends m src k
  let cur' := fun src => r.cur m src + min (bytesReq (τ.reads k) src) ((S src).size - r.cur m src)
  let bytes := fun src => r.buf m src ++ (S src).extract (r.cur m src) (cur' src)
  let batches := (srcsOf (τ.reads k)).map fun src => (⟨src, (parseFrames (bytes src)).1⟩ : (src : Member L mem) × List WireMsg)
  Dag.iterate m true d (Dag.Timing.ofReadings m (τ.readings k) d) (Dag.inboxOfWire m d cod 0 batches).1
    (envRestrict m (τ.inp k)) (r.cells m)

theorem act_fst (cod : Dag.Codecs d) (τ : WireTrace d) (k : Nat) : (τ.act cod k).1 = (τ.body cod k).1 := by
  unfold act actOf body Dag.iterateBytes Dag.iterateWire
  simp only [List.map_map, Function.comp_def]

theorem act_outs (cod : Dag.Codecs d) (τ : WireTrace d) (k : Nat) : (τ.act cod k).2.1 = (τ.body cod k).2.1 := by
  unfold act actOf body Dag.iterateBytes Dag.iterateWire
  simp only [List.map_map, Function.comp_def]

theorem act_sends (cod : Dag.Codecs d) (τ : WireTrace d) (k : Nat) :
    (τ.act cod k).2.2.1 = (τ.body cod k).2.2.flatMap fun snd => (Dag.wireEncode (τ.who k) d cod 0 snd).1 := by
  unfold act actOf body Dag.iterateBytes Dag.iterateWire
  simp only [List.map_map, Function.comp_def]

/-- The passes of activation `k`: the tick and its stutters. -/
def passes (cod : Dag.Codecs d) (τ : WireTrace d) (k : Nat) : Nat := (τ.body cod k).2.2.length

theorem passes_pos (cod : Dag.Codecs d) (τ : WireTrace d) (k : Nat) : 0 < τ.passes cod k := by
  unfold passes body Dag.iterate
  simp

/-- The pass-level step at which activation `k` starts. -/
def blockStart (cod : Dag.Codecs d) (τ : WireTrace d) : Nat → Nat
  | 0 => 0
  | k + 1 => τ.blockStart cod k + τ.passes cod k

theorem blockStart_lt_succ (cod : Dag.Codecs d) (τ : WireTrace d) (k : Nat) :
    τ.blockStart cod k < τ.blockStart cod (k + 1) :=
  Nat.lt_add_of_pos_right (τ.passes_pos cod k)

theorem blockStart_mono (cod : Dag.Codecs d) (τ : WireTrace d) {k k' : Nat} (h : k ≤ k') :
    τ.blockStart cod k ≤ τ.blockStart cod k' := by
  induction k' with
  | zero => cases Nat.le_zero.mp h; exact Nat.le_refl _
  | succ k' ih =>
    rcases Nat.lt_or_ge k (k' + 1) with hlt | hge
    · exact Nat.le_trans (ih (Nat.lt_succ_iff.mp hlt)) (Nat.le_of_lt (τ.blockStart_lt_succ cod k'))
    · cases Nat.le_antisymm h hge; exact Nat.le_refl _

theorem le_blockStart (cod : Dag.Codecs d) (τ : WireTrace d) : ∀ k, k ≤ τ.blockStart cod k
  | 0 => Nat.le_refl _
  | k + 1 => Nat.lt_of_le_of_lt (τ.le_blockStart cod k) (τ.blockStart_lt_succ cod k)

/-- The activation a pass-level step belongs to. -/
noncomputable def blockOf (cod : Dag.Codecs d) (τ : WireTrace d) (t : Nat) : Nat :=
  Nat.find (p := fun k => t < τ.blockStart cod (k + 1)) ⟨t, Nat.lt_of_lt_of_le (Nat.lt_succ_self t) (τ.le_blockStart cod (t + 1))⟩

theorem blockOf_eq (cod : Dag.Codecs d) (τ : WireTrace d) (k j : Nat) (hj : j < τ.passes cod k) :
    τ.blockOf cod (τ.blockStart cod k + j) = k := by
  have hex : ∃ k', τ.blockStart cod k + j < τ.blockStart cod (k' + 1) :=
    ⟨τ.blockStart cod k + j, Nat.lt_of_lt_of_le (Nat.lt_succ_self _) (τ.le_blockStart cod _)⟩
  show Nat.find hex = k
  have hspec : τ.blockStart cod k + j < τ.blockStart cod (Nat.find hex + 1) := Nat.find_spec hex
  have hmin : Nat.find hex ≤ k :=
    Nat.find_min' hex (by show τ.blockStart cod k + j < τ.blockStart cod k + τ.passes cod k; omega)
  apply Nat.le_antisymm hmin
  by_contra hlt
  have h1 : Nat.find hex + 1 ≤ k := Nat.succ_le_of_lt (Nat.lt_of_not_le hlt)
  have h2 := τ.blockStart_mono cod h1
  omega

/-- The list-like and slot increments of every member emptied; the registers kept. -/
def envQuietG : {Γ : Ctx L mem} → HList (Kind.delta (mem := mem)) Γ → HList (Kind.delta (mem := mem)) Γ
  | [], _ => HList.nil
  | _ :: _, Δ => (Kind.quietDeltaG _ Δ.1, envQuietG Δ.2)

theorem envRestrict_quietG (m : Member L mem) : {Γ : Ctx L mem} → (Δ : HList (Kind.delta (mem := mem)) Γ) →
    envRestrict m (envQuietG Δ) = envQuiet m (envRestrict m Δ)
  | [], _ => rfl
  | _ :: _, Δ => by
    show (Kind.restrictDelta m _ (Kind.quietDeltaG _ Δ.1), envRestrict m (envQuietG Δ.2))
      = (Kind.quietDelta m _ (Kind.restrictDelta m _ Δ.1), envQuiet m (envRestrict m Δ.2))
    rw [Kind.restrictDelta_quietG, envRestrict_quietG m Δ.2]

/-- The tick pass of activation `k`: its body on the parsed frames. -/
def tickPass (cod : Dag.Codecs d) (τ : WireTrace d) (k : Nat) :=
  let r := τ.run cod k
  let m := τ.who k
  let S := fun src => PassTrace.bytesOn (d := d) τ.who r.sends m src k
  let cur' := fun src => r.cur m src + min (bytesReq (τ.reads k) src) ((S src).size - r.cur m src)
  let bytes := fun src => r.buf m src ++ (S src).extract (r.cur m src) (cur' src)
  let batches := (srcsOf (τ.reads k)).map fun src => (⟨src, (parseFrames (bytes src)).1⟩ : (src : Member L mem) × List WireMsg)
  Dag.pass m true d (Dag.Timing.ofReadings m (τ.readings k) d) (Dag.inboxOfWire m d cod 0 batches).1
    (envRestrict m (τ.inp k)) (r.cells m)

/-- The quieted inputs of activation `k`'s settling passes. -/
def quietInp (τ : WireTrace d) (k : Nat) : HList (Kind.locDelta (τ.who k)) Γ :=
  envQuiet (τ.who k) (envRestrict (τ.who k) (τ.inp k))

/-- The quieted timing of activation `k`'s settling passes: the clock reading, no fires. -/
def quietTm (τ : WireTrace d) (k : Nat) : Dag.Timing (τ.who k) d :=
  Dag.Timing.quiet (τ.who k) d (Dag.Timing.ofReadings (τ.who k) (τ.readings k) d)

/-- The cells before pass `j` of activation `k` (`0`: before the tick). -/
def passCells (cod : Dag.Codecs d) (τ : WireTrace d) (k : Nat) : Nat → Dag.LocState (τ.who k) d
  | 0 => (τ.run cod k).cells (τ.who k)
  | j + 1 => Dag.passState (τ.who k) d (τ.quietTm k) (τ.quietInp k) (τ.tickPass cod k).1 j

/-- Whether pass `j` of activation `k` ticks: the first does; a settling pass as
`Dag.settleFlag` reads the cells before it. -/
def passFlag (cod : Dag.Codecs d) (τ : WireTrace d) (k : Nat) : Nat → Bool
  | 0 => true
  | j + 1 => Dag.flagOf (τ.who k) d (τ.passCells cod k (j + 1))

/-- Pass `j` of activation `k`: the tick, then the settling passes. -/
def passResult (cod : Dag.Codecs d) (τ : WireTrace d) (k : Nat) : Nat →
    Dag.LocState (τ.who k) d × HList (Kind.locDelta (τ.who k)) outs × Dag.Sends (τ.who k) d
  | 0 => τ.tickPass cod k
  | j + 1 => Dag.pass (τ.who k) (τ.passFlag cod k (j + 1)) d (τ.quietTm k) (Dag.Inbox.empty (τ.who k) d)
      (τ.quietInp k) (τ.passCells cod k (j + 1))

theorem passResult_fst (cod : Dag.Codecs d) (τ : WireTrace d) (k : Nat) (j : Nat) :
    (τ.passResult cod k j).1 = τ.passCells cod k (j + 1) := by
  cases j with
  | zero => rfl
  | succ j => rfl

theorem passFlag_zero (cod : Dag.Codecs d) (τ : WireTrace d) (k : Nat) : τ.passFlag cod k 0 = true := rfl

/-- **The expansion**: the pass-level trace of the activations — the activation's tick at
its block's start, then its settling passes (the same process, no bytes read, the inputs
quieted, the readings quieted; the tick flag `passFlag` the settling reads from the cells). -/
noncomputable def expand (cod : Dag.Codecs d) (τ : WireTrace d) : PassTrace d :=
  { who := fun t => τ.who (τ.blockOf cod t)
    tick := fun t => τ.passFlag cod (τ.blockOf cod t) (t - τ.blockStart cod (τ.blockOf cod t))
    inp := fun t => if t = τ.blockStart cod (τ.blockOf cod t) then τ.inp (τ.blockOf cod t) else envQuietG (τ.inp (τ.blockOf cod t))
    readings := fun t => if t = τ.blockStart cod (τ.blockOf cod t) then τ.readings (τ.blockOf cod t) else (τ.readings (τ.blockOf cod t)).quiet
    reads := fun t => if t = τ.blockStart cod (τ.blockOf cod t) then τ.reads (τ.blockOf cod t) else [] }

/-! ### The expansion runs the trace -/

/-- What activation `k'` put on its connection to `m`. -/
def sent (cod : Dag.Codecs d) (τ : WireTrace d) (k' : Nat) (m : Member L mem) : List WireMsg :=
  Dag.wireFor m d (τ.sends cod k')

/-- The messages `src` has put on its connection to `m` by activation `k`. -/
def stream (cod : Dag.Codecs d) (τ : WireTrace d) (m src : Member L mem) (k : Nat) : List WireMsg :=
  streamOf τ.who (τ.sent cod) m src k

theorem run_sends_stable (cod : Dag.Codecs d) (τ : WireTrace d) {k' : Nat} :
    ∀ {k}, k' < k → (τ.run cod k).sends k' = τ.sends cod k'
  | 0, h => absurd h (Nat.not_lt_zero _)
  | k + 1, h => by
    rcases Nat.lt_or_eq_of_le (Nat.lt_succ_iff.mp h) with hlt | rfl
    · show (if k' = k then _ else (τ.run cod k).sends k') = _
      rw [if_neg (Nat.ne_of_lt hlt)]
      exact run_sends_stable cod τ hlt
    · rfl

theorem bytesOn_eq (cod : Dag.Codecs d) (τ : WireTrace d) (k : Nat) (m src : Member L mem) :
    PassTrace.bytesOn (d := d) τ.who (τ.run cod k).sends m src k = frameMsgs (τ.stream cod m src k) := by
  unfold PassTrace.bytesOn stream streamOf sent frameMsgs
  congr 1
  apply List.flatMap_congr
  intro k' hk'
  show Dag.wireFor m d ((τ.run cod k).sends k') = _
  rw [run_sends_stable cod τ (mem_activationsOf.mp hk').1]

theorem sends_eq (cod : Dag.Codecs d) (τ : WireTrace d) (k : Nat) : τ.sends cod k = (τ.act cod k).2.2.1 := by
  show (if k = k then _ else _) = _
  rw [if_pos rfl]
  rfl

/-- A block of steps at one member contributes its passes' messages, in order. -/
theorem streamOf_block (who : Nat → Member L mem) (sent : Nat → Member L mem → List WireMsg) (m src w : Member L mem)
    (t₀ : Nat) : ∀ n, (∀ j, j < n → who (t₀ + j) = w) →
    streamOf who sent m src (t₀ + n)
      = streamOf who sent m src t₀ ++ (if w = src then ((List.range n).map fun j => sent (t₀ + j) m).flatten else [])
  | 0, _ => by simp
  | n + 1, hw => by
    rw [← Nat.add_assoc, streamOf_succ, streamOf_block who sent m src w t₀ n (fun j hj => hw j (Nat.lt_succ_of_lt hj)),
      hw n (Nat.lt_succ_self n), List.range_succ, List.map_append, List.flatten_append, List.append_assoc]
    split <;> simp

/-- A fold over a list indexed by position. -/
theorem foldl_eq_range {α β : Type} (g : β → α → β) (init : β) (st : List α) (f : Nat → α)
    (hf : ∀ j (h : j < st.length), st[j] = f j) :
    st.foldl g init = (List.range st.length).foldl (fun o j => g o (f j)) init := by
  induction st using List.reverseRecOn with
  | nil => rfl
  | append_singleton st x ih =>
    rw [List.foldl_append, List.length_append, List.length_singleton, List.range_succ, List.foldl_append,
      List.foldl_cons, List.foldl_nil, List.foldl_cons, List.foldl_nil]
    rw [ih (fun j hj => by
      have := hf j (by simp; omega)
      rwa [List.getElem_append_left hj] at this)]
    congr 1
    have := hf st.length (by simp)
    rw [List.getElem_append_right (Nat.le_refl _)] at this
    simpa using this

/-- The settling passes of activation `k`. -/
def settleOf (cod : Dag.Codecs d) (τ : WireTrace d) (k : Nat) :=
  Dag.settle (τ.who k) d (τ.quietTm k) (τ.quietInp k) (Dag.settleFuel d) (τ.tickPass cod k).1

theorem body_eq (cod : Dag.Codecs d) (τ : WireTrace d) (k : Nat) :
    τ.body cod k
      = ((τ.settleOf cod k).foldl (fun _ q => q.1) (τ.tickPass cod k).1,
         (τ.settleOf cod k).foldl (fun o q => envMerge (τ.who k) o q.2.1) (τ.tickPass cod k).2.1,
         (τ.tickPass cod k).2.2 :: (τ.settleOf cod k).map (·.2.2)) := rfl

theorem passes_eq (cod : Dag.Codecs d) (τ : WireTrace d) (k : Nat) :
    τ.passes cod k = (τ.settleOf cod k).length + 1 := by
  unfold passes
  rw [body_eq]
  simp

theorem settleOf_getElem (cod : Dag.Codecs d) (τ : WireTrace d) (k : Nat) (j : Nat) (hj : j < (τ.settleOf cod k).length) :
    (τ.settleOf cod k)[j] = τ.passResult cod k (j + 1) := by
  have := Dag.settle_getElem? (τ.who k) d (τ.quietTm k) (τ.quietInp k) (Dag.settleFuel d) (τ.tickPass cod k).1 j hj
  rw [show Dag.settle (τ.who k) d (τ.quietTm k) (τ.quietInp k) (Dag.settleFuel d) (τ.tickPass cod k).1
    = τ.settleOf cod k from rfl, List.getElem?_eq_getElem hj] at this
  exact Option.some.inj this

/-- The body's sends are the passes'. -/
theorem body_sends (cod : Dag.Codecs d) (τ : WireTrace d) (k : Nat) :
    (τ.body cod k).2.2 = (List.range (τ.passes cod k)).map fun j => (τ.passResult cod k j).2.2 := by
  rw [body_eq, passes_eq, List.range_succ_eq_map, List.map_cons, List.map_map]
  simp only
  congr 1
  apply List.ext_getElem (by simp)
  intro j h1 h2
  simp only [List.getElem_map, Function.comp_def, List.getElem_range]
  rw [settleOf_getElem cod τ k j (by simpa using h1)]

/-- The body's cells are the last pass's. -/
theorem body_fst (cod : Dag.Codecs d) (τ : WireTrace d) (k : Nat) :
    (τ.body cod k).1 = τ.passCells cod k (τ.passes cod k) := by
  rw [body_eq, passes_eq]
  show (Dag.settle (τ.who k) d (τ.quietTm k) (τ.quietInp k) (Dag.settleFuel d)
    (τ.tickPass cod k).1).foldl (fun _ q => q.1) (τ.tickPass cod k).1 = _
  rw [Dag.settle_foldl_fst]
  rfl

/-- The body's outputs are the passes', merged in order. -/
theorem body_outs (cod : Dag.Codecs d) (τ : WireTrace d) (k : Nat) :
    (τ.body cod k).2.1
      = (List.range (τ.passes cod k - 1)).foldl (fun o j => envMerge (τ.who k) o (τ.passResult cod k (j + 1)).2.1)
          (τ.tickPass cod k).2.1 := by
  rw [body_eq, passes_eq, Nat.add_sub_cancel]
  exact foldl_eq_range _ _ _ (fun j => τ.passResult cod k (j + 1)) (settleOf_getElem cod τ k)

/-! ### The expansion's passes -/

/-- The expanded trace at pass `j` of activation `k`: the process, whether it ticks, what it
reads, its inputs. -/
theorem expand_who (cod : Dag.Codecs d) (τ : WireTrace d) (k j : Nat) (hj : j < τ.passes cod k) :
    (τ.expand cod).who (τ.blockStart cod k + j) = τ.who k := by
  show τ.who (τ.blockOf cod (τ.blockStart cod k + j)) = _
  rw [τ.blockOf_eq cod k j hj]

/-- The readings of a pass are its activation's (up to the process's name). -/
theorem expand_readings (cod : Dag.Codecs d) (τ : WireTrace d) (k j : Nat) (hj : j < τ.passes cod k) :
    HEq ((τ.expand cod).readings (τ.blockStart cod k + j)) (if j = 0 then τ.readings k else (τ.readings k).quiet) := by
  show HEq (if τ.blockStart cod k + j = τ.blockStart cod (τ.blockOf cod (τ.blockStart cod k + j)) then
    τ.readings (τ.blockOf cod (τ.blockStart cod k + j)) else (τ.readings (τ.blockOf cod (τ.blockStart cod k + j))).quiet) _
  rw [τ.blockOf_eq cod k j hj]
  by_cases h0 : j = 0 <;> simp [h0]

theorem expand_tick (cod : Dag.Codecs d) (τ : WireTrace d) (k j : Nat) (hj : j < τ.passes cod k) :
    (τ.expand cod).tick (τ.blockStart cod k + j) = τ.passFlag cod k j := by
  show τ.passFlag cod (τ.blockOf cod (τ.blockStart cod k + j)) (τ.blockStart cod k + j - τ.blockStart cod (τ.blockOf cod (τ.blockStart cod k + j))) = _
  rw [τ.blockOf_eq cod k j hj, Nat.add_sub_cancel_left]

theorem expand_reads (cod : Dag.Codecs d) (τ : WireTrace d) (k j : Nat) (hj : j < τ.passes cod k) :
    (τ.expand cod).reads (τ.blockStart cod k + j) = if j = 0 then τ.reads k else [] := by
  show (if τ.blockStart cod k + j = τ.blockStart cod (τ.blockOf cod (τ.blockStart cod k + j)) then
    τ.reads (τ.blockOf cod (τ.blockStart cod k + j)) else []) = _
  rw [τ.blockOf_eq cod k j hj]
  by_cases h0 : j = 0 <;> simp [h0]

theorem expand_inp (cod : Dag.Codecs d) (τ : WireTrace d) (k j : Nat) (hj : j < τ.passes cod k) :
    (τ.expand cod).inp (τ.blockStart cod k + j) = if j = 0 then τ.inp k else envQuietG (τ.inp k) := by
  show (if τ.blockStart cod k + j = τ.blockStart cod (τ.blockOf cod (τ.blockStart cod k + j)) then
    τ.inp (τ.blockOf cod (τ.blockStart cod k + j)) else envQuietG (τ.inp (τ.blockOf cod (τ.blockStart cod k + j)))) = _
  rw [τ.blockOf_eq cod k j hj]
  by_cases h0 : j = 0 <;> simp [h0]

/-- The body of one pass of the expanded trace, at the running process as the expansion
names it: the tick (`j = 0`) reads the activation's bytes and inputs, a stutter reads nothing
and the quieted inputs. -/
theorem expand_actOf (cod : Dag.Codecs d) (τ : WireTrace d) (k j : Nat) (hj : j < τ.passes cod k) (r : RunState d) :
    (τ.expand cod).actOf cod (τ.blockStart cod k + j) r
      = (let m := (τ.expand cod).who (τ.blockStart cod k + j)
         let S := fun src => PassTrace.bytesOn (d := d) (τ.expand cod).who r.sends m src (τ.blockStart cod k + j)
         let reads := if j = 0 then τ.reads k else []
         let cur' := fun src => r.cur m src + min (bytesReq reads src) ((S src).size - r.cur m src)
         let bytes := fun src => r.buf m src ++ (S src).extract (r.cur m src) (cur' src)
         let chans := (srcsOf reads).map fun src => (⟨src, bytes src⟩ : (src : Member L mem) × ByteArray)
         Dag.passBytes m (τ.passFlag cod k j) d cod (Dag.Timing.ofReadings m ((τ.expand cod).readings (τ.blockStart cod k + j)) d) chans
           (envRestrict m (if j = 0 then τ.inp k else envQuietG (τ.inp k))) (r.cells m)) := by
  unfold PassTrace.actOf
  rw [τ.expand_tick cod k j hj, τ.expand_reads cod k j hj, τ.expand_inp cod k j hj]

/-- One pass of the expanded trace, as the activation's pass `j`: the tick (`j = 0`) reads
the activation's bytes and inputs, a stutter reads nothing and the quieted inputs. -/
theorem expand_step (cod : Dag.Codecs d) (τ : WireTrace d) (k j : Nat) (hj : j < τ.passes cod k) (r : RunState d) :
    (τ.expand cod).step cod (τ.blockStart cod k + j) r
      = (let m := τ.who k
         let S := fun src => PassTrace.bytesOn (d := d) (τ.expand cod).who r.sends m src (τ.blockStart cod k + j)
         let reads := if j = 0 then τ.reads k else []
         let cur' := fun src => r.cur m src + min (bytesReq reads src) ((S src).size - r.cur m src)
         let bytes := fun src => r.buf m src ++ (S src).extract (r.cur m src) (cur' src)
         let chans := (srcsOf reads).map fun src => (⟨src, bytes src⟩ : (src : Member L mem) × ByteArray)
         let a := Dag.passBytes m (τ.passFlag cod k j) d cod
           (Dag.Timing.ofReadings m (if j = 0 then τ.readings k else (τ.readings k).quiet) d) chans
           (envRestrict m (if j = 0 then τ.inp k else envQuietG (τ.inp k))) (r.cells m)
         { cells := fun m' => if h : m = m' then h ▸ a.1 else r.cells m'
           cur := fun m' src => if m = m' then cur' src else r.cur m' src
           buf := fun m' src => if m = m' ∧ src ∈ srcsOf reads then (parseFrames (bytes src)).2 else r.buf m' src
           sends := fun k' => if k' = τ.blockStart cod k + j then a.2.2.1 else r.sends k' }) := by
  have hb := τ.blockOf_eq cod k j hj
  simp only [PassTrace.step, PassTrace.actOf]
  -- expose the activation the pass belongs to, and identify it
  show (let m := τ.who (τ.blockOf cod (τ.blockStart cod k + j))
        let S := fun src => PassTrace.bytesOn (d := d) (τ.expand cod).who r.sends m src (τ.blockStart cod k + j)
        let cur' := fun src => r.cur m src + min (bytesReq ((τ.expand cod).reads (τ.blockStart cod k + j)) src) ((S src).size - r.cur m src)
        let bytes := fun src => r.buf m src ++ (S src).extract (r.cur m src) (cur' src)
        let chans := (srcsOf ((τ.expand cod).reads (τ.blockStart cod k + j))).map fun src => (⟨src, bytes src⟩ : (src : Member L mem) × ByteArray)
        let a := Dag.passBytes m ((τ.expand cod).tick (τ.blockStart cod k + j)) d cod
          (Dag.Timing.ofReadings m (if τ.blockStart cod k + j = τ.blockStart cod (τ.blockOf cod (τ.blockStart cod k + j)) then
            τ.readings (τ.blockOf cod (τ.blockStart cod k + j)) else (τ.readings (τ.blockOf cod (τ.blockStart cod k + j))).quiet) d) chans
          (envRestrict m ((τ.expand cod).inp (τ.blockStart cod k + j))) (r.cells m)
        { cells := fun m' => if h : m = m' then h ▸ a.1 else r.cells m'
          cur := fun m' src => if m = m' then cur' src else r.cur m' src
          buf := fun m' src => if m = m' ∧ src ∈ srcsOf ((τ.expand cod).reads (τ.blockStart cod k + j)) then (parseFrames (bytes src)).2 else r.buf m' src
          sends := fun k' => if k' = τ.blockStart cod k + j then a.2.2.1 else r.sends k' } : RunState d) = _
  rw [τ.expand_tick cod k j hj, τ.expand_reads cod k j hj, τ.expand_inp cod k j hj]
  generalize τ.blockOf cod (τ.blockStart cod k + j) = b at hb ⊢
  subst hb
  by_cases h0 : j = 0
  · subst h0
    simp only [Nat.add_zero, eq_self_iff_true, if_true]
  · have hne : τ.blockStart cod b + j ≠ τ.blockStart cod b := by omega
    simp only [if_neg h0, if_neg hne]

/-! ### The expansion's run is the trace's run, block by block -/

/-- **The correspondence at an activation boundary**: before activation `k` and before its
block's first pass, every process's cells, every cursor and tail agree, and each earlier
activation's bytes are the concatenation of its passes'. -/
structure Expands (cod : Dag.Codecs d) (τ : WireTrace d) (k : Nat) : Prop where
  cells : ∀ m, (τ.run cod k).cells m = ((τ.expand cod).run cod (τ.blockStart cod k)).cells m
  cur : ∀ m src, (τ.run cod k).cur m src = ((τ.expand cod).run cod (τ.blockStart cod k)).cur m src
  buf : ∀ m src, (τ.run cod k).buf m src = ((τ.expand cod).run cod (τ.blockStart cod k)).buf m src
  sends : ∀ k', k' < k →
    τ.sends cod k' = ((List.range (τ.passes cod k')).map fun j => (τ.expand cod).sends cod (τ.blockStart cod k' + j)).flatten

/-- The streams agree at a block start, given the earlier activations' sends. -/
theorem stream_eq (cod : Dag.Codecs d) (τ : WireTrace d) : ∀ (k : Nat),
    (∀ k', k' < k → τ.sends cod k' = ((List.range (τ.passes cod k')).map fun j => (τ.expand cod).sends cod (τ.blockStart cod k' + j)).flatten) →
    ∀ m src, (τ.expand cod).stream cod m src (τ.blockStart cod k) = τ.stream cod m src k
  | 0, _, m, src => by
    show streamOf _ _ m src 0 = streamOf _ _ m src 0
    simp [streamOf, activationsOf]
  | k + 1, hs, m, src => by
    show streamOf (τ.expand cod).who ((τ.expand cod).sent cod) m src (τ.blockStart cod k + τ.passes cod k)
      = streamOf τ.who (τ.sent cod) m src (k + 1)
    rw [streamOf_block _ _ m src (τ.who k) _ _ (fun j hj => τ.expand_who cod k j hj), streamOf_succ]
    have ih := stream_eq cod τ k (fun k' hk' => hs k' (Nat.lt_succ_of_lt hk')) m src
    show _ = τ.stream cod m src k ++ _
    rw [← ih]
    congr 1
    split
    · show ((List.range (τ.passes cod k)).map fun j => Dag.wireFor m d ((τ.expand cod).sends cod (τ.blockStart cod k + j))).flatten
        = Dag.wireFor m d (τ.sends cod k)
      rw [hs k (Nat.lt_succ_self k)]
      unfold Dag.wireFor wireForTable
      rw [List.filter_flatten, List.map_map]
      rfl
    · rfl

/-- After `j ≥ 1` passes of activation `k`'s block: the process's cells are the pass cells,
every other process is untouched, the cursors and tails are the activation's, the block's
sends so far are the passes'. -/
structure Inner (cod : Dag.Codecs d) (τ : WireTrace d) (k j : Nat) : Prop where
  cellsm : ((τ.expand cod).run cod (τ.blockStart cod k + j)).cells (τ.who k) = τ.passCells cod k j
  cells : ∀ m', m' ≠ τ.who k → ((τ.expand cod).run cod (τ.blockStart cod k + j)).cells m' = (τ.run cod k).cells m'
  cur : ∀ m' src, ((τ.expand cod).run cod (τ.blockStart cod k + j)).cur m' src = (τ.run cod (k + 1)).cur m' src
  buf : ∀ m' src, ((τ.expand cod).run cod (τ.blockStart cod k + j)).buf m' src = (τ.run cod (k + 1)).buf m' src
  sendsNew : ∀ i, i < j → ((τ.expand cod).run cod (τ.blockStart cod k + j)).sends (τ.blockStart cod k + i)
    = (Dag.wireEncode (τ.who k) d cod 0 (τ.passResult cod k i).2.2).1
  sendsOld : ∀ t, t < τ.blockStart cod k →
    ((τ.expand cod).run cod (τ.blockStart cod k + j)).sends t = ((τ.expand cod).run cod (τ.blockStart cod k)).sends t

theorem bytesReq_nil (src : Member L mem) : bytesReq ([] : List (Member L mem × Nat)) src = 0 := rfl

theorem srcsOf_nil : srcsOf ([] : List (Member L mem × Nat)) = [] := rfl

/-- The tick pass: the activation's reads on the same bytes. -/
theorem inner_one (cod : Dag.Codecs d) (τ : WireTrace d) (k : Nat) (h : τ.Expands cod k) : τ.Inner cod k 1 := by
  have hS : ∀ src, PassTrace.bytesOn (d := d) (τ.expand cod).who ((τ.expand cod).run cod (τ.blockStart cod k)).sends (τ.who k) src (τ.blockStart cod k)
      = PassTrace.bytesOn (d := d) τ.who (τ.run cod k).sends (τ.who k) src k := by
    intro src
    rw [(τ.expand cod).bytesOn_eq, τ.bytesOn_eq, τ.stream_eq cod k h.sends]
  have hstep : (τ.expand cod).run cod (τ.blockStart cod k + 1)
      = (τ.expand cod).step cod (τ.blockStart cod k + 0) ((τ.expand cod).run cod (τ.blockStart cod k)) := rfl
  constructor
  all_goals rw [hstep, τ.expand_step cod k 0 (τ.passes_pos cod k)]
  all_goals simp only [Nat.add_zero, if_true, passFlag_zero]
  · -- the process's cells: the tick pass's
    show (Dag.passBytes (τ.who k) true d cod _ _ _ _).1 = τ.passCells cod k 1
    unfold Dag.passBytes Dag.passWire
    simp only [List.map_map, Function.comp_def]
    show (Dag.pass (τ.who k) true d _ _ _ (((τ.expand cod).run cod (τ.blockStart cod k)).cells (τ.who k))).1 = _
    simp only [hS, ← h.cells, ← h.cur, ← h.buf]
    rfl
  · intro m' hm
    show (if hh : τ.who k = m' then _ else ((τ.expand cod).run cod (τ.blockStart cod k)).cells m') = _
    rw [dif_neg (Ne.symm hm), h.cells]
  · intro m' src
    show (if τ.who k = m' then _ else _) = (if τ.who k = m' then _ else _)
    simp only [hS, h.cur]
  · intro m' src
    show (if τ.who k = m' ∧ src ∈ srcsOf (τ.reads k) then _ else _) = (if τ.who k = m' ∧ src ∈ srcsOf (τ.reads k) then _ else _)
    simp only [hS, h.cur, h.buf]
  · intro i hi
    have hi0 : i = 0 := Nat.lt_one_iff.mp hi
    subst hi0
    show (if τ.blockStart cod k = τ.blockStart cod k then (Dag.passBytes (τ.who k) true d cod _ _ _ _).2.2.1 else _) = _
    rw [if_pos rfl]
    unfold Dag.passBytes Dag.passWire
    simp only [List.map_map, Function.comp_def]
    show (Dag.wireEncode (τ.who k) d cod 0 (Dag.pass (τ.who k) true d _ _ _ (((τ.expand cod).run cod (τ.blockStart cod k)).cells (τ.who k))).2.2).1 = _
    simp only [hS, ← h.cells, ← h.cur, ← h.buf]
    rfl
  · intro t ht
    show (if t = τ.blockStart cod k then _ else _) = _
    rw [if_neg (Nat.ne_of_lt ht)]

/-- A settling pass: nothing read, the pass (with the flag read from the cells) from the
previous cells, at the quieted timing. -/
theorem inner_succ (cod : Dag.Codecs d) (τ : WireTrace d) (k j : Nat) (hj1 : 1 ≤ j) (hj : j < τ.passes cod k)
    (h : τ.Inner cod k j) : τ.Inner cod k (j + 1) := by
  have hstep : (τ.expand cod).run cod (τ.blockStart cod k + (j + 1))
      = (τ.expand cod).step cod (τ.blockStart cod k + j) ((τ.expand cod).run cod (τ.blockStart cod k + j)) := rfl
  have hj0 : j ≠ 0 := Nat.pos_iff_ne_zero.mp hj1
  obtain ⟨j', rfl⟩ : ∃ j', j = j' + 1 := ⟨j - 1, by omega⟩
  constructor
  all_goals rw [hstep, τ.expand_step cod k (j' + 1) hj]
  all_goals simp only [if_neg hj0, bytesReq_nil, Nat.zero_min, Nat.add_zero, srcsOf_nil, List.map_nil,
    List.not_mem_nil, and_false, if_false]
  · show (Dag.passBytes (τ.who k) (τ.passFlag cod k (j' + 1)) d cod _ [] _ _).1 = τ.passCells cod k (j' + 1 + 1)
    unfold Dag.passBytes Dag.passWire
    simp only [List.map_nil, Dag.inboxOfWire_nil, envRestrict_quietG, Dag.Timing.ofReadings_quiet]
    show (Dag.pass (τ.who k) (τ.passFlag cod k (j' + 1)) d (τ.quietTm k) (Dag.Inbox.empty (τ.who k) d) (τ.quietInp k)
      (((τ.expand cod).run cod (τ.blockStart cod k + (j' + 1))).cells (τ.who k))).1 = _
    rw [h.cellsm]
    rfl
  · intro m' hm
    show (if hh : τ.who k = m' then _ else _) = _
    rw [dif_neg (Ne.symm hm)]
    exact h.cells m' hm
  · intro m' src
    show (if τ.who k = m' then ((τ.expand cod).run cod (τ.blockStart cod k + (j' + 1))).cur (τ.who k) src else _) = _
    split
    · rename_i hh; subst hh; exact h.cur _ src
    · exact h.cur m' src
  · intro m' src
    exact h.buf m' src
  · intro i hi
    rcases Nat.lt_or_eq_of_le (Nat.lt_succ_iff.mp hi) with hlt | rfl
    · show (if τ.blockStart cod k + i = τ.blockStart cod k + (j' + 1) then _ else _) = _
      rw [if_neg (by omega)]
      exact h.sendsNew i hlt
    · show (if τ.blockStart cod k + (j' + 1) = τ.blockStart cod k + (j' + 1) then
        (Dag.passBytes (τ.who k) (τ.passFlag cod k (j' + 1)) d cod _ [] _ _).2.2.1 else _) = _
      rw [if_pos rfl]
      unfold Dag.passBytes Dag.passWire
      simp only [List.map_nil, Dag.inboxOfWire_nil, envRestrict_quietG, Dag.Timing.ofReadings_quiet]
      show (Dag.wireEncode (τ.who k) d cod 0 (Dag.pass (τ.who k) (τ.passFlag cod k (j' + 1)) d (τ.quietTm k)
        (Dag.Inbox.empty (τ.who k) d) (τ.quietInp k)
        (((τ.expand cod).run cod (τ.blockStart cod k + (j' + 1))).cells (τ.who k))).2.2).1 = _
      rw [h.cellsm]
      rfl
  · intro t ht
    show (if t = τ.blockStart cod k + (j' + 1) then _ else _) = _
    rw [if_neg (by omega)]
    exact h.sendsOld t ht

theorem inner (cod : Dag.Codecs d) (τ : WireTrace d) (k : Nat) (h : τ.Expands cod k) :
    ∀ j, 1 ≤ j → j ≤ τ.passes cod k → τ.Inner cod k j
  | 0, h1, _ => absurd h1 (Nat.not_succ_le_zero 0)
  | 1, _, _ => τ.inner_one cod k h
  | j + 2, _, hle => τ.inner_succ cod k (j + 1) (Nat.succ_le_succ (Nat.zero_le j)) hle
      (τ.inner cod k h (j + 1) (Nat.succ_le_succ (Nat.zero_le j)) (Nat.le_of_succ_le hle))

theorem expands_zero (cod : Dag.Codecs d) (τ : WireTrace d) : τ.Expands cod 0 :=
  ⟨fun _ => rfl, fun _ _ => rfl, fun _ _ => rfl, fun _ h => absurd h (Nat.not_lt_zero _)⟩

theorem expands_succ (cod : Dag.Codecs d) (τ : WireTrace d) (k : Nat) (h : τ.Expands cod k) : τ.Expands cod (k + 1) := by
  have hin := τ.inner cod k h (τ.passes cod k) (τ.passes_pos cod k) (Nat.le_refl _)
  have hbs : τ.blockStart cod (k + 1) = τ.blockStart cod k + τ.passes cod k := rfl
  refine ⟨fun m => ?_, fun m src => ?_, fun m src => ?_, fun k' hk' => ?_⟩
  · rw [hbs]
    by_cases hm : m = τ.who k
    · subst hm
      rw [hin.cellsm]
      show (if hh : τ.who k = τ.who k then hh ▸ (τ.act cod k).1 else _) = _
      rw [dif_pos rfl]
      show (τ.act cod k).1 = _
      rw [τ.act_fst, τ.body_fst]
    · rw [hin.cells m hm]
      show (if hh : τ.who k = m then _ else (τ.run cod k).cells m) = _
      rw [dif_neg (Ne.symm hm)]
  · rw [hbs, hin.cur]
  · rw [hbs, hin.buf]
  · rcases Nat.lt_or_eq_of_le (Nat.lt_succ_iff.mp hk') with hlt | rfl
    · exact h.sends k' hlt
    · rw [τ.sends_eq, τ.act_sends, τ.body_sends, List.flatMap_def, List.map_map]
      congr 1
      apply List.map_congr_left
      intro j' hj'
      have hj'' := List.mem_range.mp hj'
      -- pass `j'`'s bytes on the expanded run: `Inner (j' + 1)`
      have hi := τ.inner cod k' h (j' + 1) (Nat.succ_le_succ (Nat.zero_le j')) hj''
      show (Dag.wireEncode (τ.who k') d cod 0 (τ.passResult cod k' j').2.2).1 = (τ.expand cod).sends cod (τ.blockStart cod k' + j')
      rw [← hi.sendsNew j' (Nat.lt_succ_self j')]
      rfl

theorem expands (cod : Dag.Codecs d) (τ : WireTrace d) : ∀ k, τ.Expands cod k
  | 0 => τ.expands_zero cod
  | k + 1 => τ.expands_succ cod k (τ.expands cod k)

/-! ### The expansion's outputs are the trace's -/

/-- Pass `j` of activation `k`'s block, on the expanded run, puts out what the activation's
pass `j` does. -/
theorem pass_outs (cod : Dag.Codecs d) (τ : WireTrace d) (k j : Nat) (hj : j < τ.passes cod k) (hE : τ.Expands cod k)
    (h : (τ.expand cod).who (τ.blockStart cod k + j) = τ.who k) :
    (h ▸ ((τ.expand cod).act cod (τ.blockStart cod k + j)).2.1 : HList (Kind.locDelta (τ.who k)) outs)
      = (τ.passResult cod k j).2.1 := by
  rw [PassTrace.act_eq_actOf, τ.expand_actOf cod k j hj]
  simp only
  have hR := τ.expand_readings cod k j hj
  generalize (τ.expand cod).readings (τ.blockStart cod k + j) = R at hR ⊢
  revert R
  generalize (τ.expand cod).who (τ.blockStart cod k + j) = w at h ⊢
  subst h
  intro R hR
  cases hR
  cases j with
  | zero =>
    have hS : ∀ src, PassTrace.bytesOn (d := d) (τ.expand cod).who ((τ.expand cod).run cod (τ.blockStart cod k)).sends (τ.who k) src (τ.blockStart cod k)
        = PassTrace.bytesOn (d := d) τ.who (τ.run cod k).sends (τ.who k) src k := by
      intro src
      rw [(τ.expand cod).bytesOn_eq, τ.bytesOn_eq, τ.stream_eq cod k hE.sends]
    simp only [Nat.add_zero, if_true, passFlag_zero]
    unfold Dag.passBytes Dag.passWire
    simp only [List.map_map, Function.comp_def]
    show (Dag.pass (τ.who k) true d _ _ _ (((τ.expand cod).run cod (τ.blockStart cod k)).cells (τ.who k))).2.1 = _
    simp only [hS, ← hE.cells, ← hE.cur, ← hE.buf]
    rfl
  | succ j' =>
    have hin := τ.inner cod k hE (j' + 1) (Nat.succ_le_succ (Nat.zero_le j')) (Nat.le_of_lt hj)
    have hj0 : j' + 1 ≠ 0 := Nat.succ_ne_zero j'
    simp only [if_neg hj0, bytesReq_nil, Nat.zero_min, Nat.add_zero, srcsOf_nil, List.map_nil]
    unfold Dag.passBytes Dag.passWire
    simp only [List.map_nil, Dag.inboxOfWire_nil, envRestrict_quietG, Dag.Timing.ofReadings_quiet]
    show (Dag.pass (τ.who k) (τ.passFlag cod k (j' + 1)) d (τ.quietTm k) (Dag.Inbox.empty (τ.who k) d) (τ.quietInp k)
      (((τ.expand cod).run cod (τ.blockStart cod k + (j' + 1))).cells (τ.who k))).2.1 = _
    rw [hin.cellsm]
    rfl

/-- A `filterMap` over a range that keeps everything is a `map`. -/
theorem filterMap_range_some {β : Type} (g : Nat → Option β) (f : Nat → β) :
    ∀ n, (∀ j, j < n → g j = some (f j)) → (List.range n).filterMap g = (List.range n).map f
  | 0, _ => rfl
  | n + 1, h => by
    rw [List.range_succ, List.filterMap_append, List.map_append, filterMap_range_some g f n (fun j hj => h j (Nat.lt_succ_of_lt hj))]
    simp [h n (Nat.lt_succ_self n)]

/-- A stream output of passes merged in order is their outputs concatenated. -/
theorem foldl_envMerge_get (m : Member L mem) (f : Nat → HList (Kind.locDelta m) outs) {ℓ : L} {α : Type}
    {iα : DecidableEq α} {ord : StrOrd} {ret : Retries} (v : Var outs (.stream ℓ α iα ord ret)) (i : Hosted m ℓ) :
    ∀ n, ((List.range n).foldl (fun o j => envMerge m o (f (j + 1))) (f 0)).get v i
      = ((List.range (n + 1)).map fun j => (f j).get v i).flatten
  | 0 => by simp
  | n + 1 => by
    rw [List.range_succ, List.foldl_append, List.foldl_cons, List.foldl_nil, envMerge_get_stream,
      foldl_envMerge_get m f v i n, List.range_succ (n := n + 1), List.map_append, List.flatten_append]
    simp

/-- **The outputs correspond**: what a stream output of process `m` has carried by activation
`T` is what it has carried on the expanded trace by `T`'s block start. -/
theorem streamOutputsAt_expand (cod : Dag.Codecs d) (τ : WireTrace d) (T : Nat) (m : Member L mem) {ℓ : L} {α : Type}
    {iα : DecidableEq α} {ord : StrOrd} {ret : Retries} (v : Var outs (.stream ℓ α iα ord ret)) (i : Hosted m ℓ) :
    τ.streamOutputsAt cod T m v i = (τ.expand cod).streamOutputsAt cod (τ.blockStart cod T) m v i := by
  induction T with
  | zero => rfl
  | succ T ih =>
    unfold streamOutputsAt PassTrace.streamOutputsAt at ih ⊢
    rw [List.range_succ, List.filterMap_append, List.flatMap_append, ih]
    show _ = ((List.range (τ.blockStart cod T + τ.passes cod T)).filterMap _).flatMap _
    rw [List.range_add, List.filterMap_append, List.flatMap_append]
    congr 1
    rw [List.filterMap_map]
    by_cases hm : τ.who T = m
    · subst hm
      simp only [List.filterMap_cons, List.filterMap_nil, dite_true, List.flatMap_cons, List.flatMap_nil, List.append_nil]
      show ((τ.act cod T).2.1).get v i = _
      rw [τ.act_outs, τ.body_outs]
      show ((List.range _).foldl (fun o j => envMerge (τ.who T) o (τ.passResult cod T (j + 1)).2.1)
        (τ.passResult cod T 0).2.1).get v i = _
      refine (foldl_envMerge_get (τ.who T) (fun j => (τ.passResult cod T j).2.1) v i (τ.passes cod T - 1)).trans ?_
      rw [Nat.sub_add_cancel (τ.passes_pos cod T)]
      rw [filterMap_range_some _ (fun j => (τ.passResult cod T j).2.1) (τ.passes cod T) ?_, List.flatMap_def, List.map_map]
      · rfl
      · intro j hj
        simp only [Function.comp_def]
        rw [dif_pos (τ.expand_who cod T j hj), τ.pass_outs cod T j hj (τ.expands cod T)]
    · simp only [List.filterMap_cons, List.filterMap_nil, dif_neg hm, List.flatMap_nil]
      rw [List.filterMap_eq_nil_iff.mpr]
      · rfl
      · intro j hj
        simp only [Function.comp_def]
        rw [dif_neg]
        rw [τ.expand_who cod T j (List.mem_range.mp hj)]
        exact hm

/-- The lock-step interleaving a wire trace induces: that of its expansion. -/
noncomputable def interleaving (cod : Dag.Codecs d) (τ : WireTrace d) : Interleaving d :=
  (τ.expand cod).interleaving cod

/-- **A wire trace's outputs are the model's**: what a stream output of process `m` has carried
by activation `T` is what the induced interleaving carries by the block start of `T`. -/
theorem streamOutputsAt_eq (cod : Dag.Codecs d) (τ : WireTrace d) (T : Nat) (m : Member L mem) {ℓ : L} {α : Type}
    {iα : DecidableEq α} {ord : StrOrd} {ret : Retries} (v : Var outs (.stream ℓ α iα ord ret)) (i : Hosted m ℓ) :
    τ.streamOutputsAt cod T m v i = (τ.interleaving cod).streamOutputsAt (τ.blockStart cod T) m v i :=
  (τ.streamOutputsAt_expand cod T m v i).trans ((τ.expand cod).streamOutputsAt_eq cod (τ.blockStart cod T) m v i)

end WireTrace

end Project

end Hydro
