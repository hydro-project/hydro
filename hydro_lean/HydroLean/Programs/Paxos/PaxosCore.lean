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
-/

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

section Body

/-! The body's **single source**: each Rust `let` is one `→ₘ` stage of the
outer cycle. The client callback is itself a staged Hydro program of the
ballot stream, so its faithful type is `→ₘ` — prefix-monotonicity of the
clients is **carried by the callback's type**, not a side hypothesis. -/

variable (variant : PaxosVariant) (f : Nat)
variable (cbM : (Fin nP → Stream (Ballot nP)) →ₘ (Fin nP → Stream P))
variable (nd : PaxosNondet P nP nA)

/-- The `leader_election` call of the body (paxos.rs:169–189). -/
def pcLEM :
    PaxosRef P nP nA →ₘ (Fin nP → MonoSing (ballotNumVO (nP := nP)))
      × (Fin nP → TSing Bool)
      × (Fin nP → TStream (P1bPayload P nP))
      × (Fin nA → MonoSing (obtVO (nP := nP))) :=
  leader_electionM variant (f + 1) (2 * f + 1) nd.le
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

/-- The `sequence_payload` call of the body (paxos.rs:206–226). -/
def pcSPM :
    PaxosRef P nP nA →ₘ (Fin nP → Stream (Nat × Option P))
      × (Fin nA → MonoSing (covVOc (P := P) (nP := nP)))
      × (Fin nP → Fin nA → Stream (Ballot nP)) :=
  let le := pcLEM variant f nd
  sequence_payloadM variant f nd.sp ∘ₘ MonoMap.pair
    (cbM ∘ₘ MonoMap.pi (fun i => pcBallotsM variant f nd i))
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
    PaxosRef P nP nA →ₘ PaxosRef P nP nA ×
      ((Fin nP → Stream (Ballot nP)) × (Fin nP → Stream (Nat × Option P))) :=
  let sp := pcSPM variant f cbM nd
  MonoMap.pair
    (-- a_log_complete_cycle.complete(…); sequencing_max_ballot_complete_cycle
     -- .complete(sequencing_max_ballots) (paxos.rs:227–239)
     PaxosRef.mkM ∘ₘ MonoMap.pair
      ((MonoMap.pi (fun i => (sp.sndOf.sndOf.member i).memoFam)).memoFam)
      ((MonoMap.pi (fun j => (sp.sndOf.fstOf.member j).vals)).memoFam))
    (MonoMap.pair
      ((MonoMap.pi (fun i => pcBallotsM variant f nd i)).memoFam)
      (sp.fstOf.memoFam))

variable (ref : PaxosRef P nP nA)

/-! ### The plain-function faces (`.f` of the single-source stages) -/

abbrev pcLE := (pcLEM variant f nd).f ref

abbrev pcBallots (i : Fin nP) : Stream (Ballot nP) :=
  (pcBallotsM variant f nd i).f ref

abbrev pcSP := (pcSPM variant f cbM nd).f ref

/-- The body's cycle completion, definitionally (the stage composition
zeta-reduces to the spec projections). -/
theorem paxos_core_body_fst :
    ((paxos_core_bodyM variant f cbM nd).f ref).1
      = ⟨memoF (fun i => memoF (fun j =>
          (pcSP variant f cbM nd ref).2.2 i j)),
         memoF (fun j =>
          ((pcSP variant f cbM nd ref).2.1 j).vals)⟩ := rfl

/-- The body's outputs, definitionally. -/
theorem paxos_core_body_snd :
    ((paxos_core_bodyM variant f cbM nd).f ref).2
      = (memoF (fun i => pcBallots variant f nd ref i),
         memoF (fun i =>
          (pcSP variant f cbM nd ref).1 i)) := rfl

end Body

/-- **paxos.rs:136–246 `paxos_core`** — the typed fixpoint (`MonoMap.fix₀`)
of the typed body. Returns `(new-leader ballots, p_to_replicas)` per
proposer. The client callback is taken at its staged type `→ₘ` (its
prefix-monotonicity is the `.mono` field — every combinator-built callback
inhabits it). -/
def paxos_core (variant : PaxosVariant) (f : Nat)
    (c_to_proposers : (Fin nP → Stream (Ballot nP)) →ₘ (Fin nP → Stream P))
    (nondet : PaxosNondet P nP nA) :
    (Fin nP → Stream (Ballot nP)) × (Fin nP → Stream (Nat × Option P)) :=
  MonoMap.fix₀ nondet.fuel PaxosRef.init
    (paxos_core_bodyM variant f c_to_proposers nondet)


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
`MonoMap.fixHist₀_chain` face of the typed fixpoint. -/
theorem paxos_core_hist_chain {k k' : Nat}
    (h : k ≤ k') :
    (MonoMap.fixHist₀ PaxosRef.init (paxos_core_bodyM variant f cbM nd) k).le
    (MonoMap.fixHist₀ PaxosRef.init (paxos_core_bodyM variant f cbM nd) k') :=
  MonoMap.fixHist₀_chain PaxosRef.init_le
    (paxos_core_bodyM variant f cbM nd) h

end Core



section Run

variable (variant : PaxosVariant) (f : Nat)
variable (cbM : (Fin nP → Stream (Ballot nP)) →ₘ (Fin nP → Stream P))
variable (nd : PaxosNondet P nP nA)

/-- The outer cycle after `k` unfoldings. -/
def pcHist (k : Nat) : PaxosRef P nP nA :=
  MonoMap.fixHist₀ PaxosRef.init (paxos_core_bodyM variant f cbM nd) k

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
      = ((spOut variant f nd.sp
          (spInputs variant f cbM nd (pcHist variant f cbM nd k))).2.1
            j).vals := by
  show (((paxos_core_bodyM variant f cbM nd).f
      (pcHist variant f cbM nd k)).1).a_log j = _
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
  have hle := (leader_electionM variant (f + 1) (2 * f + 1) nd.le).mono
    (a := ((pcHist variant f cbM nd k).sequencing_max_ballot,
           (pcHist variant f cbM nd k).a_log))
    (b := ((pcHist variant f cbM nd k').sequencing_max_ballot,
           (pcHist variant f cbM nd k').a_log))
    (leIn_le h)
  exact ⟨cbM.mono (fun i'' => pcBallots_prefix
      (paxos_core_hist_chain h) i''),
    fun i => hle.1 i, hle.2.1, hle.2.2.1, fun j => hle.2.2.2 j⟩

end Run

/-! ## The guarded run: the module contracts, instantiated and composed

Everything below is the **wiring layer** for the safety proof: the
`sequence_payload` carrier at each cycle (`pcG`), its wire discipline
discharged from `leader_election`'s contracts (`run_discipline`), the run
instantiations of the leader-view contracts, the base-case emptiness, and
the K1 composition (`run_promise_covers` — promise payloads cover chosen
keys of lower ballots, through the knot). `Safety.lean` consumes only
these and the modules' abstract witnesses (`SPEmission`, `SPChosen`) —
no module internals appear at or beyond this layer. -/

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

/-- **The wire discipline at the run** — `sequence_payload`'s carrier
requirements are exactly `leader_election`'s contracts, discharged. -/
theorem run_discipline (h : PaxosRef P nP nA) :
    SPWireDiscipline (spInputs G f cbM nd h) := by
  constructor
  · -- ballot ownership
    intro i b hb
    exact le_out_ballot_own G (f + 1) (2 * f + 1) nd.le
      h.sequencing_max_ballot h.a_log i b hb
  · -- num-monotonicity: the `Monotonic` ballot wire's `ascending`
    intro i t t' hle ht'
    exact ((pcLE G f nd h).1 i).ascending hle ht'
  · -- leader ⇒ nonempty view
    intro i t hpl hpr hl
    obtain ⟨hpr', hpb, hlen, -⟩ := le_leader_view_promise G (f + 1)
      (2 * f + 1) nd.le h.sequencing_max_ballot h.a_log i hpl hl
    intro hnil
    have hlen' : f + 1
        ≤ (((spInputs G f cbM nd h).2.2.2.1 i)[t]'hpr).length := hlen
    rw [hnil] at hlen'
    simp at hlen'
  · -- ballot stability along reigns
    intro i t ht1 hb1 hl1 hl0
    obtain ⟨hb1', heq⟩ := le_ballot_stable G (f + 1) (2 * f + 1) nd.le
      h.sequencing_max_ballot h.a_log
      (Nat.succ_le_succ (Nat.zero_le f)) i ht1 hl1 hl0
    exact heq
  · -- view pinning by ballot number
    intro i t t' hpl hpl' hpr hpr' hpb hpb' hl hl' hnum
    obtain ⟨hqr, hqr', heq⟩ := le_view_pinned G (f + 1) (2 * f + 1) nd.le
      h.sequencing_max_ballot h.a_log
      (Nat.succ_le_succ (Nat.zero_le f)) i hpl hpl' hl hl' hpb hpb' hnum
    exact heq

/-- **The leader-providers contract at the run**: at a leader tick of the
carrier's wires, `f + 1` distinct acceptors each contributed a view
payload, which is the cycle's `a_log` wire value at an acceptor tick where
the `a_max_ballot` wire carries the tick's ballot. -/
theorem run_leader_providers (k : Nat) (i : Fin nP) {t : Nat}
    (ht : t < ((pcG f cbM nd k).2.2.1 i).length)
    (hl : ((pcG f cbM nd k).2.2.1 i)[t]'ht = true) :
    ∃ (hpr : t < ((pcG f cbM nd k).2.2.2.1 i).length)
      (hpb : t < ((pcG f cbM nd k).2.1 i).length),
      ∃ S : List (Fin nA), S.Nodup ∧ f + 1 ≤ S.length ∧
        ∀ j ∈ S, ∃ v ∈ ((pcG f cbM nd k).2.2.2.1 i)[t]'hpr,
          ∃ (tj : Nat)
            (htj : tj < ((pcHist G f cbM nd k).a_log j).length),
            v = ((pcHist G f cbM nd k).a_log j)[tj]'htj ∧
            ∃ hm : tj < ((pcG f cbM nd k).2.2.2.2 j).length,
              ((pcG f cbM nd k).2.2.2.2 j)[tj]'hm
                = some (((pcG f cbM nd k).2.1 i)[t]'hpb) :=
  le_leader_providers (f + 1) (2 * f + 1) nd.le
    (pcHist G f cbM nd k).sequencing_max_ballot
    (pcHist G f cbM nd k).a_log
    (Nat.succ_le_succ (Nat.zero_le f)) i ht hl

/-- **The leader-view promise contract at the run** (the `∀`-payload
form). -/
theorem run_leader_view_promise (k : Nat) (i : Fin nP) {t : Nat}
    (ht : t < ((pcG f cbM nd k).2.2.1 i).length)
    (hl : ((pcG f cbM nd k).2.2.1 i)[t]'ht = true) :
    ∃ (hpr : t < ((pcG f cbM nd k).2.2.2.1 i).length)
      (hpb : t < ((pcG f cbM nd k).2.1 i).length),
      f + 1 ≤ (((pcG f cbM nd k).2.2.2.1 i)[t]'hpr).length ∧
      ∀ v ∈ ((pcG f cbM nd k).2.2.2.1 i)[t]'hpr,
        ∃ (j : Fin nA) (tj : Nat)
          (htj : tj < ((pcHist G f cbM nd k).a_log j).length),
          v = ((pcHist G f cbM nd k).a_log j)[tj]'htj ∧
          ∃ hm : tj < ((pcG f cbM nd k).2.2.2.2 j).length,
            ((pcG f cbM nd k).2.2.2.2 j)[tj]'hm
              = some (((pcG f cbM nd k).2.1 i)[t]'hpb) :=
  le_leader_view_promise G (f + 1) (2 * f + 1) nd.le
    (pcHist G f cbM nd k).sequencing_max_ballot
    (pcHist G f cbM nd k).a_log i ht hl

/-- **The base case**: no emission exists at the bottom cycle — a leader
tick needs a promised view payload, which is a realized `a_log` input
tick, and the bottom cycle has none. -/
theorem run_emission_zero {i : Fin nP} {slot : Nat} {b : Ballot nP}
    {v : Option P}
    (h : SPEmission G f nd.sp (pcG f cbM nd 0) i slot b v) : False := by
  obtain ⟨t, hpl, hpb, hpr, hl, -, -⟩ :=
    sp_emission_spec f nd.sp (run_discipline f cbM nd _) h
  obtain ⟨hpr', hpb', hlen, hprom⟩ :=
    run_leader_view_promise f cbM nd 0 i hpl hl
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
    (htl : t' < ((spOut G f nd.sp (pcG f cbM nd K)).2.1 j).vals.length)
    (hcov : LogCovers ((((spOut G f nd.sp
      (pcG f cbM nd K)).2.1 j).vals[t']'htl).2) slot b₁) :
    LogCovers ((((pcHist G f cbM nd (k + 1)).a_log j)[tj]'htj).2)
      slot b₁ := by
  obtain ⟨hm, hamv⟩ := ham
  have hM1 : k + 1 ≤ max (k + 1) K := Nat.le_max_left _ _
  have hM2 : K ≤ max (k + 1) K := Nat.le_max_right _ _
  -- the shared `a_max_ballot` wire at the common lift
  have hleM1 := (leader_electionM G (f + 1) (2 * f + 1) nd.le).mono
    (leIn_le (variant := G) (f := f) (cbM := cbM) (nd := nd) hM1)
  have hleM2 := (leader_electionM G (f + 1) (2 * f + 1) nd.le).mono
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
  have hlogp : ((spOut G f nd.sp (pcG f cbM nd K)).2.1 j).vals
      <+: ((spOut G f nd.sp (pcG f cbM nd (max (k + 1) K))).2.1 j).vals :=
    ((sequence_payloadM G f nd.sp).mono (pcG_le f cbM nd hM2)).2.1 j
  have hlogk : ((spOut G f nd.sp (pcG f cbM nd k)).2.1 j).vals
      <+: ((spOut G f nd.sp (pcG f cbM nd (max (k + 1) K))).2.1 j).vals := by
    refine ((sequence_payloadM G f nd.sp).mono (pcG_le f cbM nd ?_)).2.1 j
    omega
  have htlM : t' < ((spOut G f nd.sp (pcG f cbM nd (max (k + 1) K))).2.1 j).vals.length :=
    Nat.lt_of_lt_of_le htl hlogp.length_le
  have htjk : tj < ((spOut G f nd.sp (pcG f cbM nd k)).2.1 j).vals.length := by
    have := htj
    rwa [alog_succ] at this
  have htjM : tj < ((spOut G f nd.sp (pcG f cbM nd (max (k + 1) K))).2.1 j).vals.length :=
    Nat.lt_of_lt_of_le htjk hlogk.length_le
  have hcovM : LogCovers ((((spOut G f nd.sp
      (pcG f cbM nd (max (k + 1) K))).2.1 j).vals[t']'htlM).2) slot b₁ := by
    rw [← List.IsPrefix.getElem hlogp htl]
    exact hcov
  -- coverage grows along ticks (the coverage-`Monotonic` log wire)
  have hasc := ((spOut G f nd.sp (pcG f cbM nd (max (k + 1) K))).2.1 j).ascending
    (Nat.le_of_lt httj) htjM
  have hcovtj : LogCovers ((((spOut G f nd.sp
      (pcG f cbM nd (max (k + 1) K))).2.1 j).vals[tj]'htjM).2) slot b₁ :=
    hasc slot b₁ hcovM
  -- back through the knot
  have hlogeq : (((pcHist G f cbM nd (k + 1)).a_log j)[tj]'htj)
      = (((spOut G f nd.sp (pcG f cbM nd (max (k + 1) K))).2.1 j).vals[tj]'htjM) := by
    rw [List.getElem_of_eq (alog_succ (variant := G) (f := f) (cbM := cbM)
      (nd := nd) k j) htj]
    exact List.IsPrefix.getElem hlogk htjk
  rw [hlogeq]
  exact hcovtj

/-- The program's outputs are the commit views at the fuel index (the
`MonoMap.fix₀` boundary, opened once). -/
theorem paxos_core_out_eq (i : Fin nP) :
    (paxos_core G f cbM nd).2 i
      = (spOut G f nd.sp (pcG f cbM nd nd.fuel)).1 i := by
  show ((paxos_core_bodyM G f cbM nd).f
    (pcHist G f cbM nd nd.fuel)).2.2 i = _
  rw [paxos_core_body_snd]
  exact memoF_eq _ i

end GuardedRun

end HydroLean.Programs.Paxos
