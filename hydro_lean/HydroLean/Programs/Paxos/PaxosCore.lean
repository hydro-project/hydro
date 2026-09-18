import HydroLean.Programs.Paxos.LeaderElection
import HydroLean.Programs.Paxos.SequencePayload

/-!
# `paxos_core` (paxos.rs:136–246) — the protocol

The Rust function over the located surface, 1:1 with its signature:

```rust
pub fn paxos_core(proposers, acceptors, a_checkpoint,
    c_to_proposers : impl FnOnce(Stream<Ballot, …>) -> Stream<P, …>,
    config, nondet_leader, nondet_commit)
  -> (Stream<Ballot, Cluster<Proposer>>,
      Stream<(usize, Option<P>), Cluster<Proposer>, NoOrder>)
```

The body is exactly the Rust body: the two `forward_ref`s
(`sequencing_max_ballot`, `a_log`, paxos.rs:163–166) become one
`forward_ref` over the product cycle `PaxosRef`; `leader_election` and
`sequence_payload` are called with the cycle values and their outputs close
it; `just_became_leader` is the `defer_tick` derivation (:191–197); the
client callback is invoked on the new-leader ballot stream (:199–204).

`a_checkpoint` is dropped (`none`) as authorized. The `unwrap_or` at :233 is
the empty-log initial value, which is `acceptor_p2`'s scan start — the cycle
init is the *empty tick history* (no realized ticks yet), and blocked reads
model the pre-completion state.

**The verified face is on the signature**: `paxos_core` returns its
replicas output at a `SlotFunctional`-bundled type — the safety guarantee
(`variant = .guarded → nA ≤ 2 * f + 1 → SlotFunctional out`) is a field of
the return value, like the inner modules' contracts. There is no separate
headline theorem: consumers project `.2.property`, and
`#print axioms paxos_core` audits the face. -/

namespace HydroLean.Programs.Paxos

open HydroLean.Hydro

set_option synthInstance.maxSize 1024

variable {P : Type} {nP nA : Nat}

/-- The decision bundle of one `paxos_core` execution
(`nondet_leader` + `nondet_commit` + the cycle unfolding depth). -/
structure PaxosNondet (P : Type) (nP nA : Nat) where
  le : LENondet P nP nA
  sp : SPNondet P nP nA
  fuel : Nat

/-- The client callback: a staged Hydro program of the new-leader ballot
streams — prefix-monotone **by type**, a first-class monotone input wire
of `paxos_core` (fed through `MonoMap.evalM`; `paxos_core` is monotone in
it, by `MonoMap.fix`). -/
abbrev PaxosCallback (P : Type) (nP : Nat) :=
  (Fin nP → Stream (Ballot nP)) →ₘ (Fin nP → Stream P)

/-- The two `paxos_core` `forward_ref`s (paxos.rs:163–166), as one product
cycle. -/
structure PaxosRef (P : Type) (nP nA : Nat) where
  /-- `sequencing_max_ballot` (:163–164): per proposer, per acceptor. -/
  sequencing_max_ballot : Fin nP → Fin nA → Stream (Ballot nP)
  /-- `a_log` (:165–166): the within-tick atomic wire, per acceptor. -/
  a_log : Fin nA → TSing (Option Nat × LogMap P nP)

def PaxosRef.init : PaxosRef P nP nA := ⟨fun _ _ => [], fun _ => []⟩

/-- Pointwise prefix order on the cycle. -/
structure PaxosRef.le (r r' : PaxosRef P nP nA) : Prop where
  seqMax : ∀ i j, r.sequencing_max_ballot i j <+: r'.sequencing_max_ballot i j
  a_log : ∀ j, r.a_log j <+: r'.a_log j

variable [DecidableEq P]

theorem PaxosRef.le_refl (r : PaxosRef P nP nA) : r.le r :=
  ⟨fun _ _ => List.prefix_refl _, fun _ => List.prefix_refl _⟩

theorem PaxosRef.le_trans {a b c : PaxosRef P nP nA} (h1 : a.le b)
    (h2 : b.le c) : a.le c :=
  ⟨fun i j => (h1.seqMax i j).trans (h2.seqMax i j),
   fun j => (h1.a_log j).trans (h2.a_log j)⟩

theorem PaxosRef.init_le (r : PaxosRef P nP nA) : PaxosRef.init.le r :=
  ⟨fun _ _ => List.nil_prefix, fun _ => List.nil_prefix⟩

/-- The outer cycle carrier grows by `PaxosRef.le`. -/
instance : Growth (PaxosRef P nP nA) where
  le := PaxosRef.le
  le_refl := PaxosRef.le_refl
  le_trans := PaxosRef.le_trans

/-- Cycle-field projections, as stages. -/
def PaxosRef.seqMaxM :
    PaxosRef P nP nA →ₘ (Fin nP → Fin nA → Stream (Ballot nP)) :=
  ⟨PaxosRef.sequencing_max_ballot, fun h => h.seqMax⟩

def PaxosRef.a_logM :
    PaxosRef P nP nA →ₘ (Fin nA → TSing (Option Nat × LogMap P nP)) :=
  ⟨PaxosRef.a_log, fun h => h.a_log⟩

/-- Cycle construction, as a stage. -/
def PaxosRef.mkM :
    (Fin nP → Fin nA → Stream (Ballot nP))
        × (Fin nA → TSing (Option Nat × LogMap P nP))
      →ₘ PaxosRef P nP nA :=
  ⟨fun x => ⟨x.1, x.2⟩, fun h => ⟨h.1, h.2⟩⟩

/-- **Slot-functionality** — the verified face carried by `paxos_core`'s
output type: any two commits at one slot, across any two proposers, carry
the same value. -/
def SlotFunctional (out : Fin nP → Stream (Nat × Option P)) : Prop :=
  ∀ (slot : Nat) (v v' : Option P) (i i' : Fin nP),
    (slot, v) ∈ out i → (slot, v') ∈ out i' → v = v'

section Body

/-! The body's **single source**: each Rust `let` is one `→ₘ` stage of the
outer cycle. The client callback is itself a staged Hydro program of the
ballot stream, so its faithful type is `→ₘ` — prefix-monotonicity of the
clients is **carried by the callback's type**, not a side hypothesis. -/

variable (variant : PaxosVariant) (f : Nat)
variable (nd : PaxosNondet P nP nA)

/-- The `leader_election` call of the body (paxos.rs:169–189). -/
def pcLEM :
    PaxosRef P nP nA →ₘ (Fin nP → MonoSing (ballotNumVO (nP := nP)))
      × (Fin nP → TSing Bool)
      × (Fin nP → TStream (P1bPayload P nP))
      × (Fin nA → MonoSing (obtVO (nP := nP))) :=
  (leader_election variant (f + 1) (2 * f + 1) nd.le).toMonoMap
    ∘ₘ MonoMap.pair PaxosRef.seqMaxM PaxosRef.a_logM

/-- `just_became_leader` (paxos.rs:191–197). -/
def pcJustM (i : Fin nP) : PaxosRef P nP nA →ₘ TSing Bool :=
  let p_is_leader := ((pcLEM variant f nd).sndOf.fstOf).member i
  let was_not_leader := (p_is_leader.map (!·)).cons true
  (p_is_leader.zip was_not_leader).map (fun lw => lw.1 && lw.2)

/-- The new-leader ballot stream (paxos.rs:201–203, 241–243). -/
def pcBallotsM (i : Fin nP) : PaxosRef P nP nA →ₘ Stream (Ballot nP) :=
  let p_ballot := (((pcLEM variant f nd).fstOf).member i).vals
  (p_ballot.zipWith (pcJustM variant f nd i)
    (fun x b => if b then [x] else [])).flatten

/-- The `sequence_payload` call of the body (paxos.rs:206–226): the
callback is applied by `MonoMap.evalM` — a monotone wire like any
other. -/
def pcSPM :
    PaxosCallback P nP × PaxosRef P nP nA
      →ₘ (Fin nP → Stream (Nat × Option P))
      × (Fin nA → MonoSing (covVOc (P := P) (nP := nP)))
      × (Fin nP → Fin nA → Stream (Ballot nP)) :=
  let le := pcLEM variant f nd ∘ₘ MonoMap.snd
  (sequence_payload variant f nd.sp).toMonoMap ∘ₘ MonoMap.pair
    (MonoMap.evalM ∘ₘ MonoMap.pair MonoMap.fst
      (MonoMap.pi (fun i => pcBallotsM variant f nd i) ∘ₘ MonoMap.snd))
    (MonoMap.pair
      (MonoMap.pi (fun i => (le.fstOf.member i).vals))
      (MonoMap.pair
        (le.sndOf.fstOf)
        (MonoMap.pair
          (le.sndOf.sndOf.fstOf)
          (MonoMap.pi (fun j => (le.sndOf.sndOf.sndOf.member j).vals)))))

/-- One unfolding of the `paxos_core` body: close the two cycles
(paxos.rs:227–239) and return the Rust pair (:240–245). Families are
memoized at the module handoffs (`memoF`, semantically the identity) so
fixpoint iteration stays linear. Cycle preservation is its `.mono`, by
construction. -/
def paxos_core_bodyM :
    PaxosCallback P nP × PaxosRef P nP nA →ₘ PaxosRef P nP nA ×
      ((Fin nP → Stream (Ballot nP)) × (Fin nP → Stream (Nat × Option P))) :=
  let sp := pcSPM variant f nd
  MonoMap.pair
    (-- a_log_complete_cycle.complete(…); sequencing_max_ballot_complete_cycle
     -- .complete(sequencing_max_ballots) (paxos.rs:227–239)
     PaxosRef.mkM ∘ₘ MonoMap.pair
      ((MonoMap.pi (fun i => (sp.sndOf.sndOf.member i).memoFam)).memoFam)
      ((MonoMap.pi (fun j => (sp.sndOf.fstOf.member j).vals)).memoFam))
    (MonoMap.pair
      (((MonoMap.pi (fun i => pcBallotsM variant f nd i))
        ∘ₘ MonoMap.snd).memoFam)
      (sp.fstOf.memoFam))

variable (cbM : PaxosCallback P nP)
variable (ref : PaxosRef P nP nA)

/-! ### The plain-function faces (`.f` of the single-source stages) -/

abbrev pcLE := (pcLEM variant f nd).f ref

abbrev pcBallots (i : Fin nP) : Stream (Ballot nP) :=
  (pcBallotsM variant f nd i).f ref

abbrev pcSP := (pcSPM variant f nd).f (cbM, ref)

/-- The body's cycle completion, definitionally (the stage composition
zeta-reduces to the spec projections). -/
theorem paxos_core_body_fst :
    ((paxos_core_bodyM variant f nd).f (cbM, ref)).1
      = ⟨memoF (fun i => memoF (fun j =>
          (pcSP variant f nd cbM ref).2.2 i j)),
         memoF (fun j =>
          ((pcSP variant f nd cbM ref).2.1 j).vals)⟩ := rfl

/-- The body's outputs, definitionally. -/
theorem paxos_core_body_snd :
    ((paxos_core_bodyM variant f nd).f (cbM, ref)).2
      = (memoF (fun i => pcBallots variant f nd ref i),
         memoF (fun i =>
          (pcSP variant f nd cbM ref).1 i)) := rfl

end Body

section Core

variable {variant : PaxosVariant} {f : Nat}
variable {cbM : (Fin nP → Stream (Ballot nP)) →ₘ (Fin nP → Stream P)}
variable {nd : PaxosNondet P nP nA}
variable [DecidableEq P]

/-- `pcBallots` grows with the cycle (`pcBallotsM.mono`). -/
theorem pcBallots_prefix {h h' : PaxosRef P nP nA} (hr : h.le h')
    (i : Fin nP) :
    pcBallots variant f nd h i <+: pcBallots variant f nd h' i :=
  (pcBallotsM variant f nd i).mono (a := h) (b := h') hr

/-- The outer cycle histories chain along the unfolding — the generic
`MonoMap.fixHist_chain` face of the typed fixpoint. -/
theorem paxos_core_hist_chain {k k' : Nat}
    (h : k ≤ k') :
    (MonoMap.fixHist PaxosRef.init (paxos_core_bodyM variant f nd) cbM k).le
    (MonoMap.fixHist PaxosRef.init (paxos_core_bodyM variant f nd) cbM k') :=
  MonoMap.fixHist_chain PaxosRef.init_le
    (paxos_core_bodyM variant f nd) cbM h

end Core



section Run

variable (variant : PaxosVariant) (f : Nat)
variable (cbM : (Fin nP → Stream (Ballot nP)) →ₘ (Fin nP → Stream P))
variable (nd : PaxosNondet P nP nA)

/-- The outer cycle after `k` unfoldings. -/
def pcHist (k : Nat) : PaxosRef P nP nA :=
  MonoMap.fixHist PaxosRef.init (paxos_core_bodyM variant f nd) cbM k

/-- The `sequence_payload` carrier at cycle `h` — the wiring: the client
callback on the new-leader ballots and `leader_election`'s outputs. -/
def spInputs (h : PaxosRef P nP nA) : SPG P nP nA :=
  (cbM.f (fun i' => pcBallots variant f nd h i'),
   fun i' => ((pcLE variant f nd h).1 i').vals,
   (pcLE variant f nd h).2.1,
   (pcLE variant f nd h).2.2.1,
   fun j => ((pcLE variant f nd h).2.2.2 j).vals)

/-- The base cycle has no realized `a_log` ticks. -/
theorem alog_zero (j : Fin nA) :
    (pcHist variant f cbM nd 0).a_log j = [] := rfl

/-- **The `a_log` knot, opened at the signature**: the cycle's `a_log` wire
at index `k + 1` IS `sequence_payload`'s published log output at index `k`
(write-before-ack and the log-entry provenance transfer through this
equality — never assumed). -/
theorem alog_succ (k : Nat) (j : Fin nA) :
    (pcHist variant f cbM nd (k + 1)).a_log j
      = (((sequence_payload variant f nd.sp).f
          (spInputs variant f cbM nd (pcHist variant f cbM nd k))).2.1 j).vals := by
  show (((paxos_core_bodyM variant f nd).f
      (cbM, pcHist variant f cbM nd k)).1).a_log j = _
  rw [paxos_core_body_fst]
  show memoF _ j = _
  rw [memoF_eq]
  rfl

/-! ### Wire growth along the chain -/

variable {variant f nd}
variable {cbM}

theorem leIn_le {k k' : Nat} (h : k ≤ k') :
    ((pcHist variant f cbM nd k).sequencing_max_ballot,
      (pcHist variant f cbM nd k).a_log)
      ⊑ ((pcHist variant f cbM nd k').sequencing_max_ballot,
        (pcHist variant f cbM nd k').a_log) := by
  have := paxos_core_hist_chain (variant := variant) (f := f)
    (cbM := cbM) (nd := nd) h
  exact ⟨this.seqMax, this.a_log⟩

/-- The `sequence_payload` carrier grows along the chain (the client
callback's and `leader_election`'s prefix-monotonicity, composed —
type-derived). -/
theorem spInputs_le {k k' : Nat} (h : k ≤ k') :
    spInputs variant f cbM nd (pcHist variant f cbM nd k)
      ⊑ spInputs variant f cbM nd (pcHist variant f cbM nd k') := by
  have hle := (leader_election variant (f + 1) (2 * f + 1) nd.le).mono
    (a := ((pcHist variant f cbM nd k).sequencing_max_ballot,
           (pcHist variant f cbM nd k).a_log))
    (b := ((pcHist variant f cbM nd k').sequencing_max_ballot,
           (pcHist variant f cbM nd k').a_log))
    (leIn_le h)
  exact ⟨cbM.mono (fun i'' => pcBallots_prefix
      (paxos_core_hist_chain h) i''),
    fun i => hle.1 i, hle.2.1, hle.2.2.1, fun j => hle.2.2.2 j⟩

end Run

/-! ## The guarded run: the verified module faces, composed

Everything below is the **wiring layer** for the safety proof: the
`leader_election` and `sequence_payload` verified instances at each cycle
(`pcLEV`, `spV` — their ghost fields ARE the former handoff layer: the
wire discipline is passed **by constructor projection**, field-by-field,
checked by the elaborator), the base-case emptiness, the K1 composition
(`run_promise_covers`), and the final assembly (`emission_chosen_agree`,
`fix_slot_functional` — the proof `paxos_core` carries on its output
type). No module internals appear at or beyond this layer. -/

section GuardedRun

/-- The guarded variant, fixed for the safety wiring. -/
local notation "G" => PaxosVariant.guarded

variable (f : Nat)
variable (cbM : (Fin nP → Stream (Ballot nP)) →ₘ (Fin nP → Stream P))
variable (nd : PaxosNondet P nP nA)

/-- The `sequence_payload` carrier at cycle index `k` (the run wires). -/
abbrev pcG (k : Nat) : SPG P nP nA :=
  spInputs G f cbM nd (pcHist G f cbM nd k)

/-- The carrier grows along the chain. -/
theorem pcG_le {k k' : Nat} (h : k ≤ k') :
    pcG f cbM nd k ⊑ pcG f cbM nd k' :=
  spInputs_le h

/-- `leader_election`'s guarantees at cycle `h` (`.ensures` of the single
artifact — requirements: none). -/
abbrev pcLEV (h : PaxosRef P nP nA) :
    LEEnsures G (f + 1) h.a_log
      ((leader_election G (f + 1) (2 * f + 1) nd.le).f
        (h.sequencing_max_ballot, h.a_log)) :=
  (leader_election G (f + 1) (2 * f + 1) nd.le).ensures
    (h.sequencing_max_ballot, h.a_log)

/-- `sequence_payload`'s guarantees at cycle index `k` (`.ensures` of the
single artifact). -/
abbrev spV (k : Nat) :
    SPEnsures G f nd.sp (pcG f cbM nd k)
      ((sequence_payload G f nd.sp).f (pcG f cbM nd k)) :=
  (sequence_payload G f nd.sp).ensures (pcG f cbM nd k)

/-- `sequence_payload`'s **requirements** at cycle index `k` are
`leader_election`'s guarantees, **projected** — composition is
application; a missing requirement is a type error here. -/
theorem pcReq (k : Nat) : SPRequires (pcG f cbM nd k) :=
  let lev := pcLEV f nd (pcHist G f cbM nd k)
  ⟨lev.own,
   fun i _ _ hle ht' =>
     ((pcLE G f nd (pcHist G f cbM nd k)).1 i).ascending hle ht',
   lev.lead_ne (Nat.succ_le_succ (Nat.zero_le f)),
   lev.stable (Nat.succ_le_succ (Nat.zero_le f)),
   lev.pinned (Nat.succ_le_succ (Nat.zero_le f))⟩

/-- **The base case**: no emission exists at the bottom cycle — a leader
tick needs a promised view payload, which is a realized `a_log` input
tick, and the bottom cycle has none. -/
theorem run_emission_zero {i : Fin nP} {slot : Nat} {b : Ballot nP}
    {v : Option P}
    (h : SPEmission G f nd.sp (pcG f cbM nd 0) i slot b v) : False := by
  obtain ⟨t, hpl, hpb, hpr, hl, -, -⟩ :=
    (spV f cbM nd 0).emission_spec (pcReq f cbM nd 0) rfl h
  obtain ⟨hpr', hpb', hlen, hprom⟩ :=
    (pcLEV f nd (pcHist G f cbM nd 0)).view_promise i hpl hl
  have hpos : 0 < (((pcG f cbM nd 0).2.2.2.1 i)[t]'hpr').length :=
    Nat.lt_of_lt_of_le (Nat.succ_pos f) hlen
  obtain ⟨v₀, hv₀⟩ := List.exists_mem_of_length_pos hpos
  obtain ⟨j, tj, htj, -, -⟩ := hprom v₀ hv₀
  rw [alog_zero] at htj
  exact absurd htj (by simp)

/-- **The K1 composition (promise coverage of chosen keys)**: a view
payload promised at ballot `b₂` (cycle `k + 1`, acceptor tick `tj`) covers
every key chosen at a strictly lower ballot `b₁` at the same acceptor:
the shared `Monotonic` `a_max_ballot` wire orders the vote's tick strictly
before the promise's, the published log's coverage-`Monotonic` type
carries the vote's write-before-ack coverage forward, and the promise
payload IS the published log at its tick (the knot, `alog_succ`). -/
theorem run_promise_covers {k K : Nat} {j : Fin nA} {b₁ b₂ : Ballot nP}
    (hlt : b₁.blt b₂ = true)
    {tj : Nat} (htj : tj < ((pcHist G f cbM nd (k + 1)).a_log j).length)
    (ham : ∃ hm : tj < ((pcG f cbM nd (k + 1)).2.2.2.2 j).length,
      ((pcG f cbM nd (k + 1)).2.2.2.2 j)[tj]'hm = some b₂)
    {slot : Nat} {t' : Nat}
    (hta : t' < ((pcG f cbM nd K).2.2.2.2 j).length)
    (hama : ((pcG f cbM nd K).2.2.2.2 j)[t']'hta = some b₁)
    (htl : t' < (((sequence_payload G f nd.sp).f (pcG f cbM nd K)).2.1 j).vals.length)
    (hcov : LogCovers (((((sequence_payload G f nd.sp).f (pcG f cbM nd K)).2.1 j).vals[t']'htl).2) slot b₁) :
    LogCovers ((((pcHist G f cbM nd (k + 1)).a_log j)[tj]'htj).2)
      slot b₁ := by
  obtain ⟨hm, hamv⟩ := ham
  have hM1 : k + 1 ≤ max (k + 1) K := Nat.le_max_left _ _
  have hM2 : K ≤ max (k + 1) K := Nat.le_max_right _ _
  -- the shared `a_max_ballot` wire at the common lift
  have hleM1 := (leader_election G (f + 1) (2 * f + 1) nd.le).mono
    (leIn_le (variant := G) (f := f) (cbM := cbM) (nd := nd) hM1)
  have hleM2 := (leader_election G (f + 1) (2 * f + 1) nd.le).mono
    (leIn_le (variant := G) (f := f) (cbM := cbM) (nd := nd) hM2)
  have pAm1 : ((pcG f cbM nd (k + 1)).2.2.2.2 j : TSing (Option (Ballot nP)))
      <+: (pcG f cbM nd (max (k + 1) K)).2.2.2.2 j := by
    have := hleM1.2.2.2 j
    exact this
  have pAm2 : ((pcG f cbM nd K).2.2.2.2 j : TSing (Option (Ballot nP)))
      <+: (pcG f cbM nd (max (k + 1) K)).2.2.2.2 j := by
    have := hleM2.2.2.2 j
    exact this
  have hmM : tj < ((pcG f cbM nd (max (k + 1) K)).2.2.2.2 j).length :=
    Nat.lt_of_lt_of_le hm pAm1.length_le
  have htaM : t' < ((pcG f cbM nd (max (k + 1) K)).2.2.2.2 j).length :=
    Nat.lt_of_lt_of_le hta pAm2.length_le
  have hamvM : ((pcG f cbM nd (max (k + 1) K)).2.2.2.2 j)[tj]'hmM = some b₂ := by
    rw [← List.IsPrefix.getElem pAm1 hm]
    exact hamv
  have hamaM : ((pcG f cbM nd (max (k + 1) K)).2.2.2.2 j)[t']'htaM = some b₁ := by
    rw [← List.IsPrefix.getElem pAm2 hta]
    exact hama
  -- the vote's tick strictly precedes the promise's (`Monotonic` wire)
  have httj : t' < tj := by
    have hb2 : ((pcLE G f nd (pcHist G f cbM nd (max (k + 1) K))).2.2.2
        j).vals[tj]'hmM = some b₂ := hamvM
    have hb1 : ((pcLE G f nd (pcHist G f cbM nd (max (k + 1) K))).2.2.2
        j).vals[t']'htaM = some b₁ := hamaM
    refine MonoSing.tick_lt_of_not_le
      ((pcLE G f nd (pcHist G f cbM nd (max (k + 1) K))).2.2.2 j) hmM htaM ?_
    rw [hb2, hb1]
    intro hble
    exact Ballot.ble_blt_asymm (hble : b₂.ble b₁ = true) hlt
  -- coverage transports to the lift and forward to the promise tick
  have hlogp : (((sequence_payload G f nd.sp).f (pcG f cbM nd K)).2.1 j).vals
      <+: (((sequence_payload G f nd.sp).f (pcG f cbM nd (max (k + 1) K))).2.1 j).vals :=
    ((sequence_payload G f nd.sp).mono (pcG_le f cbM nd hM2)).2.1 j
  have hlogk : (((sequence_payload G f nd.sp).f (pcG f cbM nd k)).2.1 j).vals
      <+: (((sequence_payload G f nd.sp).f (pcG f cbM nd (max (k + 1) K))).2.1 j).vals := by
    refine ((sequence_payload G f nd.sp).mono (pcG_le f cbM nd ?_)).2.1 j
    omega
  have htlM : t' < (((sequence_payload G f nd.sp).f (pcG f cbM nd (max (k + 1) K))).2.1 j).vals.length :=
    Nat.lt_of_lt_of_le htl hlogp.length_le
  have htjk : tj < (((sequence_payload G f nd.sp).f
      (pcG f cbM nd k)).2.1 j).vals.length := by
    have := htj
    rwa [alog_succ] at this
  have htjM : tj < (((sequence_payload G f nd.sp).f (pcG f cbM nd (max (k + 1) K))).2.1 j).vals.length :=
    Nat.lt_of_lt_of_le htjk hlogk.length_le
  have hcovM : LogCovers (((((sequence_payload G f nd.sp).f (pcG f cbM nd (max (k + 1) K))).2.1 j).vals[t']'htlM).2) slot b₁ := by
    rw [← List.IsPrefix.getElem hlogp htl]
    exact hcov
  -- coverage grows along ticks (the coverage-`Monotonic` log wire)
  have hasc := (((sequence_payload G f nd.sp).f (pcG f cbM nd (max (k + 1) K))).2.1 j).ascending
    (Nat.le_of_lt httj) htjM
  have hcovtj : LogCovers (((((sequence_payload G f nd.sp).f (pcG f cbM nd (max (k + 1) K))).2.1 j).vals[tj]'htjM).2) slot b₁ :=
    hasc slot b₁ hcovM
  -- back through the knot
  have hlogeq : (((pcHist G f cbM nd (k + 1)).a_log j)[tj]'htj)
      = ((((sequence_payload G f nd.sp).f (pcG f cbM nd (max (k + 1) K))).2.1 j).vals[tj]'htjM) := by
    rw [List.getElem_of_eq (alog_succ (variant := G) (f := f) (cbM := cbM)
      (nd := nd) k j) htj]
    exact List.IsPrefix.getElem hlogk htjk
  rw [hlogeq]
  exact hcovtj

/-- The fixpoint's outputs are the commit views at the fuel index (the
`MonoMap.fix` boundary, opened once). -/
theorem fix_out_eq (i : Fin nP) :
    (((MonoMap.fix nd.fuel PaxosRef.init
        (paxos_core_bodyM G f nd)).f cbM)).2 i
      = ((sequence_payload G f nd.sp).f (pcG f cbM nd nd.fuel)).1 i := by
  show ((paxos_core_bodyM G f nd).f
    (cbM, pcHist G f cbM nd nd.fuel)).2.2 i = _
  rw [paxos_core_body_snd]
  exact memoF_eq _ i

set_option maxHeartbeats 1000000 in
/-- **K4 — the ONE protocol induction** (`Nat`-induction over the unfolding
depth; the provenance regress steps `k + 1 → k` through the `a_log` knot):
any emission at a slot chosen at a strictly lower ballot carries the chosen
key's value. Every step consumes a named module contract at the run. -/
theorem emission_chosen_agree
    (hnA : nA ≤ 2 * f + 1) (K : Nat) (slot : Nat)
    {i₁ : Fin nP} {b₁ : Ballot nP} {val₁ : Option P}
    (hb₁own : b₁.proposerId = i₁)
    (hm₁ : SPEmission G f nd.sp (pcG f cbM nd K) i₁ slot b₁ val₁)
    (hch : SPChosen f (pcG f cbM nd K)
      ((sequence_payload G f nd.sp).f (pcG f cbM nd K)) slot b₁) :
    ∀ (k : Nat) (i₂ : Fin nP) (b₂ : Ballot nP) (v₂ : Option P),
      b₁.blt b₂ = true →
      SPEmission G f nd.sp (pcG f cbM nd k) i₂ slot b₂ v₂ →
      v₂ = val₁ := by
  intro k
  induction k with
  | zero =>
    intro i₂ b₂ v₂ hlt hm₂
    exact absurd hm₂ (fun hc => run_emission_zero f cbM nd hc)
  | succ k ih =>
    intro i₂ b₂ v₂ hlt hm₂
    -- the emission's leader tick, ballot, and covered-value clause
    obtain ⟨t, hpl, hpb, hpr, hl, hbal, hcovd⟩ :=
      (spV f cbM nd (k + 1)).emission_spec (pcReq f cbM nd (k + 1)) rfl hm₂
    -- `f + 1` distinct promisers at the same tick
    obtain ⟨hpr', hpb', S, hSnd, hSlen, hSprov⟩ :=
      (pcLEV f nd (pcHist G f cbM nd (k + 1))).providers rfl
        (Nat.succ_le_succ (Nat.zero_le f)) i₂ hpl hl
    -- `f + 1` distinct voters for the chosen key
    obtain ⟨C, hCnd, hClen, hCvote⟩ := hch
    -- quorum intersection
    obtain ⟨j, hjS, hjC⟩ := nodup_inter_of_length hSnd hCnd (by omega)
    obtain ⟨vj, hvj, tj, htj, hpay, hmj, hmax⟩ := hSprov j hjS
    obtain ⟨t', hta, htl, hama, hcov⟩ := hCvote j hjC
    -- the promise carries the emission's ballot on the shared max wire
    have hbal' : ((pcG f cbM nd (k + 1)).2.1 i₂)[t]'hpb' = b₂ := hbal
    have hmax' : ((pcG f cbM nd (k + 1)).2.2.2.2 j)[tj]'hmj
        = some (((pcG f cbM nd (k + 1)).2.1 i₂)[t]'hpb') := hmax
    rw [hbal'] at hmax'
    -- K1: the promise payload covers the chosen key
    have hcovj := run_promise_covers f cbM nd hlt htj ⟨hmj, hmax'⟩
      hta hama htl hcov
    obtain ⟨e₀, hfind, he₀⟩ := hcovj
    -- the covering entry lives in the emission tick's view
    have hpaymem : (slot, e₀) ∈ (vj.2 : LogMap P nP) := by
      rw [hpay]
      exact LogMap.find?_mem hfind
    have hvj' : vj ∈ ((pcG f cbM nd (k + 1)).2.2.2.1 i₂)[t]'hpr := hvj
    have hE : (slot, e₀)
        ∈ ((((pcG f cbM nd (k + 1)).2.2.2.1 i₂)[t]'hpr).map
            Prod.snd).flatten :=
      List.mem_flatten.mpr ⟨vj.2, List.mem_map.mpr ⟨vj, hvj', rfl⟩, hpaymem⟩
    -- the emission's value is the view's merged max-ballot entry
    obtain ⟨best, hval, hdom, hbest⟩ := hcovd e₀ hE
    have hb₁best : b₁.ble best.ballot = true := by
      rcases he₀ with heq₀ | hlt₀
      · rw [← heq₀]
        exact hdom
      · exact Ballot.ble_trans (Ballot.ble_of_blt hlt₀) hdom
    -- the merged entry's carrier is a promise payload; regress to index `k`
    obtain ⟨l', hl', hbestl⟩ := List.mem_flatten.mp hbest
    obtain ⟨v', hv'mem, rfl⟩ := List.mem_map.mp hl'
    obtain ⟨hprv, hpbv, -, hprom⟩ :=
      (pcLEV f nd (pcHist G f cbM nd (k + 1))).view_promise i₂ hpl hl
    have hv'mem' : v' ∈ ((pcG f cbM nd (k + 1)).2.2.2.1 i₂)[t]'hprv :=
      hv'mem
    obtain ⟨j', tj', htj', hpay', -⟩ := hprom v' hv'mem'
    -- the promise payload is the previous index's published log (the knot)
    have htlk : tj' < (((sequence_payload G f nd.sp).f (pcG f cbM nd k)).2.1 j').vals.length := by
      have := htj'
      rwa [alog_succ] at this
    have hEk : (slot, best)
        ∈ (((((sequence_payload G f nd.sp).f
            (pcG f cbM nd k)).2.1 j').vals[tj']'htlk).2 : LogMap P nP) := by
      have hbl : (slot, best) ∈ (v'.2 : LogMap P nP) := hbestl
      rw [hpay'] at hbl
      have hgeq : (((pcHist G f cbM nd (k + 1)).a_log j')[tj']'htj')
          = (((sequence_payload G f nd.sp).f (pcG f cbM nd k)).2.1 j').vals[tj']'htlk := by
        rw [List.getElem_of_eq (alog_succ (variant := G) (f := f)
          (cbM := cbM) (nd := nd) k j') htj']
      rw [← hgeq]
      exact hbl
    have hm₃ := (spV f cbM nd k).log_entry_spec (pcReq f cbM nd k) rfl htlk hEk
    -- dichotomy at the merged entry's ballot
    rcases Ballot.eq_or_blt_of_ble hb₁best with hbeq | hblt
    · -- the entry is the chosen ballot's own emission: functionality
      rw [← hbeq, hb₁own] at hm₃
      have hm₃M := SPEmission.mono G f nd.sp
        (pcG_le f cbM nd (Nat.le_max_left k K)) hm₃
      have hm₁M := SPEmission.mono G f nd.sp
        (pcG_le f cbM nd (Nat.le_max_right k K)) hm₁
      have hbv := (spV f cbM nd (max k K)).emission_functional
        (pcReq f cbM nd (max k K)) rfl
        (pcG_le f cbM nd (Nat.le_refl _)) hm₃M hm₁M
      rw [hval, hbv]
    · -- strictly higher entry ballot: the induction hypothesis at `k`
      have := ih best.ballot.proposerId best.ballot best.value hblt hm₃
      rw [hval, this]

/-- **Slot-functionality of the guarded fixpoint** (deliverable (c)): over
the **full** decision space (every batching/snapshot/shuffle decision,
every client payload timing, every cycle-unfolding depth in `nd`), any two
commits at one slot — across any two proposers — carry the same value.
This is the proof obligation `paxos_core` pays to return its replicas
stream at the `SlotFunctional`-bundled type. -/
theorem fix_slot_functional (hnA : nA ≤ 2 * f + 1) :
    SlotFunctional
      (((MonoMap.fix nd.fuel PaxosRef.init
        (paxos_core_bodyM G f nd)).f cbM)).2 := by
  intro slot v v' i i' h h'
  rw [fix_out_eq f cbM nd i] at h
  rw [fix_out_eq f cbM nd i'] at h'
  obtain ⟨b, hbo, hem, hch⟩ :=
    (spV f cbM nd nd.fuel).commit_spec (pcReq f cbM nd nd.fuel) rfl h
  obtain ⟨b', hbo', hem', hch'⟩ :=
    (spV f cbM nd nd.fuel).commit_spec (pcReq f cbM nd nd.fuel) rfl h'
  by_cases hbb : b.blt b' = true
  · -- b < b': the higher emission over the `b`-chosen slot
    have hagree := emission_chosen_agree f cbM nd hnA nd.fuel slot
      hbo hem hch nd.fuel i' b' v' hbb hem'
    exact hagree.symm
  · by_cases hbb' : b'.blt b = true
    · -- b' < b: symmetric
      exact emission_chosen_agree f cbM nd hnA nd.fuel slot
        hbo' hem' hch' nd.fuel i b v hbb' hem
    · -- equal ballots: same proposer, same key ⇒ the same emission
      have hble : b'.ble b = true :=
        Ballot.ble_of_not_blt (fun hc => hbb hc)
      have heq : b' = b := by
        rcases Ballot.eq_or_blt_of_ble hble with heq | hblt
        · exact heq
        · exact absurd hblt (fun hc => hbb' hc)
      subst heq
      have hii : i' = i := by rw [← hbo, ← hbo']
      subst hii
      exact (spV f cbM nd nd.fuel).emission_functional
        (pcReq f cbM nd nd.fuel) rfl
        (pcG_le f cbM nd (Nat.le_refl _)) hem hem'

end GuardedRun

/-- **paxos.rs:136–246 `paxos_core`** — THE HEADLINE (deliverable (c)):
the single verified artifact, a `Verified` map **from the client callback**
(a first-class monotone input: prefix-monotone clients by type,
`paxos_core` monotone in them by `MonoMap.fix`) to the Rust return pair
`(new-leader ballots, p_to_replicas)`, with the safety guarantee on the
signature —

  *if running with the bugfix flag (`variant = .guarded`) and `f + 1`
  quorums can intersect (`nA ≤ 2 * f + 1`), commits are slot-functional:
  any two commits at one slot, across any two proposers, over the **full**
  decision space (every batching/snapshot/shuffle decision, every client
  payload timing, every cycle-unfolding depth), carry the same value.*

There is no separate headline theorem: consumers project the guarantee
(`.ensures cb rfl hnA`), and `#print axioms paxos_core` audits the face
(the proof is a field of the definition). The B1 send-once and B2
recommit-once guards, the paxos.rs:862 key uniqueness, and the
paxos.rs:186–189 leader-ballot stability (`leader_election`'s `stable`
guarantee) are all **derived** (the guarded variant plus the election
protocol's own trigger gate), not assumed. The faithful variant falsifies
the same statement (`Paxos/Falsification.lean`).

The value stays definitionally the 1:1 transcription (the two paxos.rs
`forward_ref`s closed by the typed fixpoint over the body; the proof is
attached at the boundary and never touches the computation). -/
def paxos_core (variant : PaxosVariant) (f : Nat)
    (nondet : PaxosNondet P nP nA) :
    Verified (PaxosCallback P nP)
      ((Fin nP → Stream (Ballot nP)) × (Fin nP → Stream (Nat × Option P)))
      (fun _cb out =>
        variant = .guarded → nA ≤ 2 * f + 1 → SlotFunctional out.2) :=
  Verified.ofMono
    (-- the two forward_refs (paxos.rs:163–166), closed over the body
     MonoMap.fix nondet.fuel PaxosRef.init
      (paxos_core_bodyM variant f nondet))
    (fun cb hv hnA => by
      subst hv
      exact fix_slot_functional f cb nondet hnA)

end HydroLean.Programs.Paxos
