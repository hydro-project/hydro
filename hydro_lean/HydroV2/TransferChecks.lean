import HydroV2.TransferTheory

/-!
# HydroV2 · executable adversary witnesses (the step machine at work)

The ruling-1 coverage checklist, as concrete machine runs over tiny
programs (`#guard`s — the build fails if any behavior is lost):

1. **cross-sender interleaving** emerges from delivery timing at
   `values` fan-in — two cursor schedules, two merged orders, same
   multiset;
2. **unbounded latency / member silence** — a stalled cursor never
   delivers; downstream sees only the live sender, forever;
3. **per-pair FIFO** — every delivered view is a prefix of the sent
   order (TCP; reordering is only ever cross-sender);
4. **stutter ticks are observable** — same deliveries, finer tick
   skeleton ⇒ more (empty) batches and duplicated snapshot reads;
5. **`AtLeastOnce` duplication is born at sampling** — an unchanged
   latest sampled twice is a consecutive stutter, and the `StutterSeq`
   coupling destutters exactly it;
6. **the fixpoint race** — a stalled message is *overtaken* by the
   offspring of a younger message through a cycle knot, and the
   machine's raced order is covered by the denotational fixpoint
   (derived decisions exist);
7. **snapshot intermediates** — tick pacing changes which accumulation
   prefixes a snapshot exposes (real nondeterminism, captured), while
   permuting arrivals *within* one delivery step leaves unordered fold
   reads unchanged (quotiented symmetrically, by license);
8. **`StabilizesAt` is non-vacuous** — a delivered wire computably
   attains its stabilization point (the end-of-time hypothesis is
   inhabited);
9. **tick presence is independent across cluster members** — two
   members of one location run *different* skeletons: at one and the
   same global step, member `0` ticks (with content) while member `1`
   has no tick entry at all, visible in batch counts and in a
   tick-counting op.
-/

namespace HydroV2
namespace TransferChecks

abbrev twoMem : Unit → Nat := fun _ => 2

/-- The step machine over two members, every member ticking every
step. -/
abbrev S : HydroSem Unit twoMem :=
  SchedSem Unit twoMem (fun _ _ _ => true)

/-- The same machine, every member ticking on even steps only. -/
abbrev SHalf : HydroSem Unit twoMem :=
  SchedSem Unit twoMem (fun _ _ t => t % 2 == 0)

/-- Member `0` sends `[10, 11]`, member `1` sends `[20, 21]`. -/
def src : S.Stream () Nat .totalOrder .exactlyOnce :=
  fun j => StepHist.const (if j = (0 : Fin 2) then [10, 11] else [20, 21])

/-- Sender `0` delivered from step 1, sender `1` from step 2. -/
def dA : Fin 2 → Fin 2 → Nat → Nat :=
  fun _i j t => if j = (0 : Fin 2) then t else t - 1

/-- Sender `1` delivered from step 1, sender `0` from step 2. -/
def dB : Fin 2 → Fin 2 → Nat → Nat :=
  fun _i j t => if j = (0 : Fin 2) then t - 1 else t

/-- Sender `1` silent forever. -/
def dSilent : Fin 2 → Fin 2 → Nat → Nat :=
  fun _i j t => if j = (0 : Fin 2) then t else 0

def mergedA : S.Stream () Nat .noOrder .exactlyOnce :=
  S.values (S.broadcast dA src)
def mergedB : S.Stream () Nat .noOrder .exactlyOnce :=
  S.values (S.broadcast dB src)
def mergedSilent : S.Stream () Nat .noOrder .exactlyOnce :=
  S.values (S.broadcast dSilent src)

/-! ### 1 · cross-sender interleaving is schedule-controlled -/

#guard (mergedA 0).view 4 = [10, 11, 20, 21]
#guard (mergedB 0).view 4 = [20, 10, 21, 11]
#guard (mergedA 0).view 4 ≠ (mergedB 0).view 4
#guard Multiset.ofList ((mergedA 0).view 4)
  = Multiset.ofList ((mergedB 0).view 4)

/-! ### 2 · unbounded latency / silence -/

#guard (mergedSilent 0).view 8 = [10, 11]

/-! ### 3 · per-pair FIFO (delivered views are prefixes of sent) -/

#guard (List.range 6).all (fun t =>
  ((S.broadcast (p := ()) dA src 0 1).view t).isPrefixOf [20, 21])
#guard (List.range 6).all (fun t =>
  ((S.broadcast (p := ()) dB src 0 0).view t).isPrefixOf [10, 11])

/-! ### 4 · stutter ticks are observable -/

def batchesAll := S.batch mergedA ()
def batchesHalf := SHalf.batch mergedA ()

#guard (batchesAll 0 5).length = 6      -- a batch per step…
#guard (batchesHalf 0 5).length = 3     -- …vs per even step
#guard ([] : List Nat) ∈ batchesAll 0 5 -- stutter tick: empty batch
#guard (batchesAll 0 5).flatten = (batchesHalf 0 5).flatten

theorem addComm : FoldOkP .noOrder .exactlyOnce
    (fun (s x : Nat) => s + x) :=
  fun s x y => Nat.add_right_comm s x y

def sumFold := S.fold (fun s x => s + x) 0 addComm mergedA
def snapAll := S.snapshot sumFold ()
def snapHalf := SHalf.snapshot (SHalf.fold (fun s x => s + x) 0
  addComm mergedA) ()

-- content exhausted by step 2; ticks 3, 4 re-read the same total
#guard snapAll 0 4 = [0, 10, 41, 62, 62]

/-! ### 5 · `AtLeastOnce` is born at sampling; the coupling destutters -/

/-- A latest that never changes, realized tick by tick. -/
def lat : S.TickSingleton () (Option Nat) .unbounded :=
  fun _i step => List.replicate (min (step + 1) 4) (some 5)

def sampled := S.sample_every lat (fun _ => [0, 2])

#guard (sampled 0).view 5 = [5, 5]           -- concrete duplicate
#guard destutter ((sampled 0).view 5) = [5]  -- the quotient's view

/-! ### 6 · the fixpoint race (offspring overtakes a stalled elder) -/

/-- Member `0` sends `[1]` (delivered at step 1); member `1` sends
`[2]` (stalled until step 5). -/
def srcRace : S.Stream () Nat .totalOrder .exactlyOnce :=
  fun j => StepHist.const (if j = (0 : Fin 2) then [1] else [2])

def dRace : Fin 2 → Fin 2 → Nat → Nat :=
  fun _i j t => if j = (0 : Fin 2) then t else t - 4

def baseRace : S.Stream () Nat .noOrder .exactlyOnce :=
  S.values (S.broadcast dRace srcRace)

/-- The knot: every arrival below 100 breeds an offspring `+100`. -/
def raceBody (s : S.Stream () Nat .noOrder .exactlyOnce) :
    S.Stream () Nat .noOrder .exactlyOnce :=
  S.union baseRace
    (S.filterMap s (fun _ x => if x < 100 then some (x + 100) else none))

def raceKnot := S.fix_stream () raceBody

-- message 1 arrives (step 1), its offspring 101 is bred through the
-- cycle and lands BEFORE the older message 2 (stalled until step 5):
#guard (raceKnot 0).view 7 = [1, 101, 2, 102]
#guard ((raceKnot 0).view 7).idxOf 101 < ((raceKnot 0).view 7).idxOf 2

-- the knot-boundary floor is visible: the offspring lands exactly one
-- step after its cause enters the knot wire…
#guard 1 ∈ (raceKnot 0).view 2 ∧ 101 ∉ (raceKnot 0).view 2
#guard 101 ∈ (raceKnot 0).view 3
-- …and the stalled elder still loses the race under the floors
-- (delays are floors, not exactness: lateness stays unbounded).
#guard 2 ∉ (raceKnot 0).view 5 ∧ 101 ∈ (raceKnot 0).view 5

-- …and the raced order is covered by the denotational fixpoint
-- (derived decisions exist: the machine run sits below `Values`' knot):
abbrev V : HydroSem Unit twoMem := Values Unit twoMem
def vBase : V.Stream () Nat .noOrder .exactlyOnce := fun _ => {1, 2}
def vBody (v : V.Stream () Nat .noOrder .exactlyOnce) :
    V.Stream () Nat .noOrder .exactlyOnce :=
  V.union vBase
    (V.filterMap v (fun _ x => if x < 100 then some (x + 100) else none))
def vKnot := V.fix_stream (3 : Nat) vBody

#guard Multiset.ofList ((raceKnot 0).view 7) ≤ vKnot 0

/-! ### 7 · snapshot intermediates: pacing is captured, micro-order is
quotiented -/

#guard snapAll 0 5 ≠ snapHalf 0 5          -- pacing changes the reads
#guard [10, 20].foldl (· + ·) 0 = [20, 10].foldl (· + ·) 0
#guard Multiset.ofList [10, 20] = Multiset.ofList [20, 10]

/-! ### 8 · `StabilizesAt` is non-vacuous (end-of-time is inhabited) -/

def stabWire : StepHist Nat := (StepHist.const [1, 2]).deliver (fun s => s)

-- computably attained: views 2, 3, …, 8 all equal the limit
#guard (List.range 7).all (fun t => stabWire.view (2 + t) == stabWire.view 2)
#guard stabWire.view 2 = [1, 2]

/-- …and provably: the generous delivery of a constant wire stabilizes
at its content length (an instance of the end-of-time kit). -/
example : StabilizesAt stabWire (0 + ([1, 2] : List Nat).length + 1) :=
  (stabilizesAt_const [1, 2]).deliver_id

/-! ### 9 · tick presence is independent across cluster members -/

/-- One location, two members, *different* skeletons: member `0` ticks
every step, member `1` only every third step. -/
abbrev SSkew : HydroSem Unit twoMem :=
  SchedSem Unit twoMem
    (fun _ i t => if i = (0 : Fin 2) then true else t % 3 == 0)

def batchesSkew := SSkew.batch mergedA ()

-- the joint behavior: at global step 1, member 0 ticks — with content —
-- while member 1 has NO tick entry at that step at all
#guard batchesSkew 0 1 = [[], [10]]  -- ticks at steps 0 and 1
#guard batchesSkew 1 1 = [[]]        -- tick at step 0 only
#guard (batchesSkew 0 1).length ≠ (batchesSkew 1 1).length

-- tick-COUNTING ops see the skew directly (tick presence is semantics,
-- not presentation): the same pulse list, consumed per member skeleton
def pulsesSkew := SSkew.source_interval_batch (ℓ := ())
  (fun _ => [true, true, true, true])

#guard pulsesSkew 0 4 = [true, true, true, true]  -- 5 ticks by step 4
#guard pulsesSkew 1 4 = [true, true]              -- ticks at 0 and 3
#guard (pulsesSkew 0 4).length ≠ (pulsesSkew 1 4).length

end TransferChecks
end HydroV2
