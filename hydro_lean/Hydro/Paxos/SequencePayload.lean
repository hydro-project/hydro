import Hydro.Paxos.Recommit
import Hydro.Paxos.IndexPayloads
import Hydro.Paxos.AcceptorP2
import Hydro.Std.Quorum
import Hydro.Std.RequestResponse
import Hydro.HydroDef

/-!
# `sequence_payload` (paxos.rs:678–774)

The leader's sequencing pipeline: reconcile the quorum's logs
(`recommit_after_leader_election`), index the incoming payload batch
(`index_payloads`, gated by `p_is_leader`), stamp everything with the
tick's ballot, ship P2as to the acceptors (`acceptor_p2`), and collect
per-`(slot, ballot)` quorums of `Ok` P2bs (`collect_quorum` at `f + 1`
of `2f + 1`), joining freshly quorum'd keys with the leader's own sent
metadata (`join_responses`) into the replica stream.

Program composition is by module invocation; **proofs never re-enter
callees** — every callee's behavior enters through its contract face
(`RCEnsures`, `IPEnsures`, `AP2Ensures`, `CQEnsures`, `JREnsures`), and
every sequencing fact is proven INLINE in the program as a ghost at the
program point it concerns (FINDINGS D63/D64). The contract (`SPEnsures`)
is stated over the module's OUTPUT wires — the published acceptor logs
and the replica commits — never over the leader's internal sent traffic:
*ensures are over outputs, never internal wires; consumers consume
ensures, period* (D64). What `paxos_core`'s K4 reads: a log entry or a
commit at `(slot, b)` pins the leader tick of `b` whose recovered view
characterizes its value (`LeaderOpen`), and entries/commits at one
`(slot, b)` agree.

Before the program: only what its contract face needs — the decision
records, `SPChosen`, `LeaderOpen`/`CommitWitness`, the gate's one input
shape `SPGateIn`, and `SPEnsures` (its requirement is the shared
`LeaderDiscipline`, the gate's loop invariant the shared `OnceInv`). **The program
starts at `hydro def sequence_payload`.**
-/

namespace Hydro

variable {L : Type} {mem : L → Nat} {P : Type} [DecidableEq P]

/-! ## Prerequisites for the contract face -/

/-- `sequence_payload`'s `nondet!` sites (paxos.rs:596–734), incl. the
P2a consumption it hands to `acceptor_p2`. -/
structure SPDec (H : HydroSem L mem) (nP nA : Nat) (P : Type)
    [DecidableEq P] (ckα : Type) [DecidableEq ckα]
    (ckord : StrOrd) where
  /-- `c_to_proposers.batch(&proposer_tick, nondet_commit)`: payload
  slice sizes. -/
  payloadBatch : H.OrdBatchDec nP
  /-- `acceptor_p2`'s bundle (the P2a batch + the checkpoint
  snapshot, paxos.rs:819/:829). -/
  ap2 : AP2Dec H nP nA P ckα ckord
  /-- `collect_quorum`'s `use::batch(responses, nondet!(…))`
  (quorum.rs:97) on the keyed P2b acks: consumed ack increments. -/
  cqBatch : H.BatchDec nP
    ((Nat × Ballot nP) × Except (Option (Ballot nP)) Unit)
  /-- `join_responses`'s `use::batch(responses, nondet!(…))`
  (request_response.rs:23) on the freshly quorum'd keys. The metadata
  side is `atomic` (paxos.rs:764–770) — same-tick availability, not a
  decision. -/
  jrBatch : H.BatchDec nP ((Nat × Ballot nP) × Unit)

/-- **`sequence_payload`'s adversary-side (sched-det) bundle** (`Unit`
at `Values`; see `Sem.lean`'s classification table), nested by owning
module like `SPDec`. -/
structure SPSched (H : HydroSem L mem) (nP nA : Nat) (P : Type)
    [DecidableEq P] where
  /-- `p_to_acceptors_p2a.broadcast(&acceptors, TCP…)`: P2a delivery
  cursors. (The Rust site's `nondet!` is `nondet_membership` —
  unmodeled under closed membership; delivery itself is unmarked.) -/
  p2aCh : H.TransportDec nA nP
  /-- `acceptor_p2`'s bundle (the P2b ack transport). -/
  ap2 : AP2Sched H nP nA

/-- The trivial bundle at the denotation. -/
def SPSched.triv {nP nA : Nat} {P : Type} [DecidableEq P] :
    SPSched (Values L mem) nP nA P := ⟨(), .triv⟩

/-- A **chosen key**: `f + 1` distinct acceptors each carry a vote —
the `a_max_ballot` input holds exactly the ballot at a tick whose
published `a_log` output already covers the slot at that ballot
(write-before-ack, on the output wire). -/
def SPChosen {nA nP : Nat} (f : Nat)
    (ck : Fin nA → Trace (Option Nat))
    (mx : Fin nA → Trace (Option (Ballot nP)))
    (alog : Fin nA → Trace (ALog P nP))
    (slot : Nat) (b : Ballot nP) : Prop :=
  ∃ C : List (Fin nA), C.Nodup ∧ f + 1 ≤ C.length ∧
    ∀ j ∈ C, ∃ (t : Nat) (hta : t < (mx j).length),
      (mx j)[t]'hta = some b
      ∧ (t < (ck j).length →
          ∃ htl : t < (alog j).length,
            LogCovers ((alog j)[t]'htl).2 slot b)

/-- **A leader opens a value**: `b`'s owner led at some tick carrying `b`
with input view `view`, and `v` is characterized against the view at
`slot` — the view's max-ballot entry there (if any) has value `v` and
dominates every accepted entry at the slot. The characterization of a
proposal `paxos_core`'s K4 consumes, stated over `leader_election`'s
output wires (ballot / leading / view) at the owner. -/
def LeaderOpen {nP : Nat} (pb : Trace (Ballot nP)) (pl : Trace Bool)
    (p1bs : Trace (Multiset (ALog P nP)))
    (b : Ballot nP) (slot : Nat) (v : Option P) : Prop :=
  ∃ (t : Nat) (view : Multiset (ALog P nP)),
    pl[t]? = some true ∧ pb[t]? = some b ∧ p1bs[t]? = some view ∧
    ∀ e₀ : LogValue P nP, (slot, e₀) ∈ rcEntries view →
      ∃ best : LogValue P nP,
        (slot, best) ∈ logView (rcEntries view) ∧ v = best.value
        ∧ e₀.ballot.ble best.ballot = true

/-- **A commit's witness ballot**: a committed `(slot, v)` at proposer
`i` was chosen at an owned ballot `b` the leader opened `v` at, and every
published log entry at `(slot, b)` carries `v`. -/
structure CommitWitness {nA nP : Nat} (f : Nat)
    (pb : Fin nP → Trace (Ballot nP)) (pl : Fin nP → Trace Bool)
    (p1bs : Fin nP → Trace (Multiset (ALog P nP)))
    (ck : Fin nA → Trace (Option Nat))
    (mx : Fin nA → Trace (Option (Ballot nP)))
    (alog : Fin nA → Trace (ALog P nP))
    (i : Fin nP) (slot : Nat) (v : Option P) (b : Ballot nP) : Prop where
  owned : b.proposerId = i
  chosen : SPChosen f ck mx alog slot b
  opened : LeaderOpen (pb i) (pl i) (p1bs i) b slot v
  entries_agree : ∀ (j : Fin nA) {t : Nat} {lg : ALog P nP} {e : LogValue P nP},
    (alog j)[t]? = some lg → (slot, e) ∈ lg.2 → e.ballot = b → e.value = v

/-- One gate input per tick: (view, ((ballot, leading), led last tick)). -/
abbrev SPGateIn (P : Type) (nP : Nat) :=
  Multiset (ALog P nP) × ((Ballot nP × Bool) × Bool)

/-- What `sequence_payload` **ensures**, over the `Values` denotation,
against its inputs — on its OUTPUT wires (`p_to_replicas`, the published
`a_log`s). Preconditions are per-field (`LeaderDiscipline` =
`leader_election`'s guarantee on the ballot/leading/view wires); the
faithful variant's guarded fields claim nothing. -/
structure SPEnsures {ckα : Type} [DecidableEq ckα] {ckord : StrOrd}
    (variant : PaxosVariant) (prop acc : L) (f : Nat)
    (cp : Fin (mem prop) → List P)
    (ck : Fin (mem acc) → Trace (Option Nat))
    (pb : Fin (mem prop) → Trace (Ballot (mem prop)))
    (pl : Fin (mem prop) → Trace Bool)
    (p1bs : Fin (mem prop) → Trace (Multiset (ALog P (mem prop))))
    (mx : Fin (mem acc) → Trace (Option (Ballot (mem prop))))
    (dec : SPDec (Values L mem) (mem prop) (mem acc) P ckα ckord)
    (out : (Fin (mem prop) → Multiset (Nat × Option P))
      × (Fin (mem acc) → Trace (ALog P (mem prop)))
      × (Fin (mem prop) → Multiset (Ballot (mem prop)))) : Prop where
  /-- **A log entry is a leader's opened value** (guarded): a published
  entry `(slot, ⟨b, v⟩)` pins the leader tick of `b` (at `b`'s owner)
  whose input view characterizes `v`. -/
  log_entry_open : LeaderDiscipline (mem prop) P pb pl p1bs → variant = .guarded →
    ∀ (j : Fin (mem acc)) {t : Nat} {lg : ALog P (mem prop)} {slot : Nat}
    {e : LogValue P (mem prop)},
    (out.2.1 j)[t]? = some lg → (slot, e) ∈ lg.2 →
    LeaderOpen (pb e.ballot.proposerId) (pl e.ballot.proposerId)
      (p1bs e.ballot.proposerId) e.ballot slot e.value
  /-- **Entries at one key agree** (guarded): two published entries at
  `(slot, b)`, on any acceptors at any ticks, carry one value — the
  leader sent the key once. -/
  log_entry_agree : LeaderDiscipline (mem prop) P pb pl p1bs → variant = .guarded →
    ∀ (j j' : Fin (mem acc)) {t t' : Nat} {lg lg' : ALog P (mem prop)} {slot : Nat}
    {e e' : LogValue P (mem prop)},
    (out.2.1 j)[t]? = some lg → (slot, e) ∈ lg.2 →
    (out.2.1 j')[t']? = some lg' → (slot, e') ∈ lg'.2 →
    e.ballot = e'.ballot → e.value = e'.value
  /-- **Commit** (guarded): every emitted `(slot, value)` of
  `p_to_replicas` has a witness ballot — owned, chosen, opened by the
  leader, agreed by every published entry at the key. -/
  commit_spec : LeaderDiscipline (mem prop) P pb pl p1bs →
    variant = .guarded → ∀ {i : Fin (mem prop)} {slot : Nat}
    {v : Option P}, (slot, v) ∈ out.1 i →
    ∃ b : Ballot (mem prop), CommitWitness f pb pl p1bs ck mx out.2.1 i slot v b
  /-- **Distinct commits are at distinct ballots** (guarded): two
  commits at one slot with different values have witness ballots that
  differ — one proposer never commits two values at one ballot. -/
  commit_distinct : LeaderDiscipline (mem prop) P pb pl p1bs →
    variant = .guarded → ∀ {i i' : Fin (mem prop)} {slot : Nat}
    {v v' : Option P}, (slot, v) ∈ out.1 i → (slot, v') ∈ out.1 i' → v ≠ v' →
    ∃ b b' : Ballot (mem prop), b ≠ b'
      ∧ CommitWitness f pb pl p1bs ck mx out.2.1 i slot v b
      ∧ CommitWitness f pb pl p1bs ck mx out.2.1 i' slot v' b'
  /-- **The published clock**: the log wire ticks at most as often as the
  checkpoint input (its tick zip). -/
  log_len_le_ck : ∀ (j : Fin (mem acc)),
    (out.2.1 j).length ≤ (ck j).length
  /-- **Coverage ascent**: the published log only gains coverage along
  ticks (the accumulated entry pool grows; `logView` keeps per-slot
  maxima). -/
  log_covers_mono : ∀ {j : Fin (mem acc)} {t t' : Nat} (h : t ≤ t')
    (htl' : t' < (out.2.1 j).length) {slot : Nat}
    {b : Ballot (mem prop)},
    LogCovers ((out.2.1 j)[t]'(Nat.lt_of_le_of_lt h htl')).2 slot b →
    LogCovers ((out.2.1 j)[t']'htl').2 slot b

/-! ## The program -/

/-- **paxos.rs:678–774 `sequence_payload`** over proposers `prop` and
acceptors `acc`. Returns (`p_to_replicas`, `a_log`,
`fail_ballots`). -/
hydro def sequence_payload {ckα : Type} [DecidableEq ckα]
    {ckord : StrOrd} {ckret : Retries}
    (H : HydroSem L mem) (variant : PaxosVariant)
    (prop acc : L)
    (c_to_proposers : H.Stream prop P .totalOrder .exactlyOnce)
    (a_checkpoint : H.Singleton acc ckα (Option Nat) ckord ckret
      .unbounded)
    (p_ballot : H.Ticked prop (Ballot (mem prop)))
    (p_is_leader : H.Ticked prop Bool)
    (p_relevant_p1bs :
      H.TickStream prop (ALog P (mem prop)) .noOrder .exactlyOnce)
    (f : Nat)
    (a_max_ballot :
      H.Ticked acc (Option (Ballot (mem prop))))
    (dec : SPDec H (mem prop) (mem acc) P ckα ckord)
    (sched : SPSched H (mem prop) (mem acc) P) :
    (H.Stream prop (Nat × Option P) .noOrder .exactlyOnce
      × H.Ticked acc (ALog P (mem prop))
      × H.Stream prop (Ballot (mem prop)) .noOrder .exactlyOnce)
  ensures out => SPEnsures variant prop acc f c_to_proposers
    (fun j => a_checkpoint j (dec.ap2.ckSnap j))
    p_ballot p_is_leader p_relevant_p1bs a_max_ballot
    dec out :=
  -- GUARDED (B2 fix): recommit (and rebase) once per ballot, on
  -- becoming leader — the accepted view is gated to empty otherwise.
  -- FAITHFUL: the gate is the identity (fires at every nonempty view).
  -- (No Rust line: the fix's own `sliced!` block, FINDINGS B2.)
  tick (state recommittedAt : Option (Ballot (mem prop)) := none)
      (input view := p_relevant_p1bs)
      (input bl := H.zipTick p_ballot p_is_leader)
      (input ledLast := H.defer_tick false p_is_leader)
      -- the loop invariant: once per ballot (`OnceInv` — the register names
      -- the last fire's ballot), under the ballot wire's ownership/ascent
      -- (input facts here, so premises)
      (invariant ((rcGated : List (Multiset (ALog P (mem prop))))
          (recommittedAt : Option (Ballot (mem prop)))
          (view : Trace (Multiset (ALog P (mem prop))))
          (bl : Trace (Ballot (mem prop) × Bool)) (ledLast : Trace Bool)) =>
        ∀ (me : Fin (mem prop)),
          (∀ x ∈ Trace.zip view (Trace.zip bl ledLast),
            (x : SPGateIn P (mem prop)).2.1.1.proposerId = me) →
          List.Pairwise (fun (x y : SPGateIn P (mem prop)) => x.2.1.1.num ≤ y.2.1.1.num)
            (Trace.zip view (Trace.zip bl ledLast)) →
          OnceInv (fun x : SPGateIn P (mem prop) => x.2.1.1)
            (fun g : Multiset (ALog P (mem prop)) => g ≠ 0) variant.recommitOnce
            recommittedAt (Trace.hist (Trace.zip view (Trace.zip bl ledLast)) rcGated)) :=
    -- fire ⇔ nonempty view ∧ (faithful ∨
    --   (leading ∧ ¬ led last tick ∧ ballot ≠ recommittedAt))
    let nonempty := H.bsMap (H.bcount view) (fun n => decide (n ≠ 0))
    let fire := H.bsMap
      (H.bsZip nonempty (H.bsZip recommittedAt (H.bsZip bl ledLast)))
      (fun (ne, (ra, ((b, l), d))) =>
        ne && (!variant.recommitOnce || (l && !d && !decide (ra = some b))))
    rebind (recommittedAt :=
      H.bsMap (H.bsZip fire (H.bsZip recommittedAt bl))
        (fun (f, (ra, (b, _))) => if f then some b else ra))
    yield (rcGated := H.bfilterIf view fire)
    -- the loop obligations: the empty run, and ONE TICK — FIRE (the view
    -- goes through; the register names this ballot, which no earlier
    -- fire carries) or HOLD (nothing emitted, the register carries)
    prove init := fun _i _me _ _ => OnceInv.init,
      tick := fun i _n _out st v_t bl_t d_t hv hb hd hlen ih me hown hmono => by
        obtain ⟨b, l⟩ := bl_t
        have hx : (Trace.zip (view i) (Trace.zip (bl i) (ledLast i)))[_n]?
            = some (v_t, ((b, l), d_t)) :=
          Trace.getElem?_zip_eq_some.mpr ⟨hv, Trace.getElem?_zip_eq_some.mpr ⟨hb, hd⟩⟩
        have ih' := ih me hown hmono
        simp only [rcGated_step, den]
        by_cases hfire : (decide (Multiset.card v_t ≠ 0)
            && (!variant.recommitOnce || (l && !d_t && !decide (st = some b)))) = true
        · rw [if_pos hfire, hfire, poolFilterIf_true]
          refine OnceInv.fire hx hlen (fun y hy => by rw [hown y hy, hown _ (Trace.mem_of_read hx)])
            hmono ih' (fun hro heq => by simp [hro, heq] at hfire) ?_
          simp only [Bool.and_eq_true, decide_eq_true_eq, ne_eq, Multiset.card_eq_zero] at hfire
          exact hfire.1
        · rw [if_neg hfire, Bool.eq_false_iff.mpr hfire, poolFilterIf_false,
            PoolBot_noOrder_exactlyOnce]
          exact OnceInv.hold hx hlen ih' (fun h => h rfl);
  -- **the gate's register, named** (`ra i t` = `recommittedAt` before tick
  -- `t`): read, stepped, stalled, and its invariant at every tick — the
  -- construct's `_reg`, the one place the block's fold is read
  ghost obtain ⟨ra, hra_zero, hgate_at, hra_succ, -, -, -, hgate_inv⟩ := hrcGated_reg
  -- **the gate at a tick, decoded**: it fires iff the view is nonempty and
  -- (faithful, or: fresh leader tick whose ballot the register does not
  -- name); a fire passes the view, a hold passes nothing
  ghost have hfire : ∀ (i : Fin (mem prop)) {t : Nat}
      {g v : Multiset (ALog P (mem prop))} {b : Ballot (mem prop)} {l d : Bool},
      (rcGated i)[t]? = some g → (p_relevant_p1bs i)[t]? = some v →
      (p_ballot i)[t]? = some b → (p_is_leader i)[t]? = some l →
      (false :: p_is_leader i)[t]? = some d →
      (g ≠ 0 ↔ v ≠ 0 ∧ (variant.recommitOnce = false
          ∨ (l = true ∧ d = false ∧ ra i t ≠ some b))) ∧ (g ≠ 0 → g = v) :=
    fun i {t g v b l d} hg hv hb hl hd => by
    obtain ⟨v', bl', d', hv', hbl', hd', rfl⟩ := (hgate_at i t g).mp hg
    have hbl'' : (Trace.zip (p_ballot i) (p_is_leader i))[t]? = some bl' := hbl'
    obtain ⟨hb', hl'⟩ := Trace.getElem?_zip_eq_some'.mp hbl''
    obtain rfl := Trace.read_inj hv hv'
    obtain rfl := Trace.read_inj hb hb'
    obtain rfl := Trace.read_inj hl hl'
    obtain rfl := Trace.read_inj hd hd'
    obtain ⟨b, l⟩ := bl'
    simp only [rcGated_step, den]
    by_cases hc : (decide (Multiset.card v ≠ 0)
        && (!variant.recommitOnce || (l && !d && !decide (ra i t = some b)))) = true
    · rw [hc, poolFilterIf_true]
      simp only [Bool.and_eq_true, Bool.or_eq_true, decide_eq_true_eq,
        Bool.not_eq_eq_eq_not, Bool.not_true, ne_eq, decide_eq_false_iff_not,
        Multiset.card_eq_zero] at hc
      exact ⟨⟨fun _ => ⟨hc.1, hc.2.elim Or.inl (fun h => Or.inr ⟨h.1.1, h.1.2, h.2⟩)⟩,
        fun _ => hc.1⟩, fun _ => rfl⟩
    · rw [Bool.eq_false_iff.mpr hc, poolFilterIf_false, PoolBot_noOrder_exactlyOnce]
      simp only [Bool.and_eq_true, Bool.or_eq_true, decide_eq_true_eq,
        Bool.not_eq_eq_eq_not, Bool.not_true, ne_eq, decide_eq_false_iff_not,
        Multiset.card_eq_zero] at hc
      exact ⟨⟨fun h => absurd rfl h, fun h => absurd
        ⟨h.1, h.2.elim Or.inl (fun h' => Or.inr ⟨⟨h'.1, h'.2.1⟩, h'.2.2⟩)⟩ hc⟩,
        fun h => absurd rfl h⟩
  -- the gate's reads: tick `t` is on the gate wire iff on its inputs
  ghost have hgate_reads : ∀ (i : Fin (mem prop)) {t : Nat}
      {v : Multiset (ALog P (mem prop))} {b : Ballot (mem prop)} {l : Bool},
      (p_relevant_p1bs i)[t]? = some v → (p_ballot i)[t]? = some b →
      (p_is_leader i)[t]? = some l →
      ∃ g d, (rcGated i)[t]? = some g ∧ (false :: p_is_leader i)[t]? = some d :=
    fun i {t v b l} hv hb hl => by
    obtain ⟨d, hd⟩ : ∃ d, (false :: p_is_leader i)[t]? = some d :=
      ⟨_, List.getElem?_eq_getElem (by
        simp only [List.length_cons]; exact Nat.lt_succ_of_lt (Trace.read_lt hl))⟩
    exact ⟨_, d, (hgate_at i t _).mpr ⟨v, (b, l), d, hv,
      Trace.getElem?_zip_eq_some'.mpr ⟨hb, hl⟩, hd, rfl⟩, hd⟩
  -- let (p_log_to_recommit, p_max_slot) =
  --   recommit_after_leader_election(p_relevant_p1bs, p_ballot.clone(), f);
  let rc := recommit_after_leader_election H prop rcGated p_ballot f
  ghost have hrc := recommit_after_leader_election.ensures prop rcGated p_ballot f
  -- c_to_proposers.batch(proposer_tick, nondet!(nondet_commit))
  --   .filter_if(p_is_leader.clone())
  tick (input c_batch := H.batch_ordered c_to_proposers dec.payloadBatch)
      (input p_is_leader := p_is_leader) :=
    yield (payload_batch := H.bfilterIf c_batch p_is_leader);
  -- let indexed_payloads = index_payloads(p_max_slot, …);
  let indexed_payloads := index_payloads H prop rc.2 payload_batch
  ghost have hip := index_payloads.ensures prop rc.2 payload_batch
  -- let payloads_to_send = indexed_payloads
  --   .cross_singleton(p_ballot.clone())
  --   .map(q!(|((slot, payload), ballot)| ((slot, ballot), Some(payload))))
  --   .chain(p_log_to_recommit)        [MinOrder: TotalOrder ⊓ NoOrder]
  --   .filter_if(p_is_leader)
  --   .all_ticks_atomic();
  tick (input indexed_payloads := indexed_payloads)
      (input p_log_to_recommit := H.flattenUnordered rc.1)
      (input p_ballot := p_ballot) (input p_is_leader := p_is_leader) :=
    yield (payloads_to_send :=
      H.bfilterIf
        (H.bchain
          (H.bweakenOrder
            (H.bmap (H.bcrossSingleton indexed_payloads p_ballot)
              (fun ((slot, payload), ballot) => ((slot, ballot), some payload))))
          p_log_to_recommit)
        p_is_leader);
  -- **the send, read at a tick**: a leader tick sends its indexed payloads
  -- stamped with the ballot, plus the recommit list; a non-leader tick
  -- sends nothing
  ghost have hsend_at : ∀ (i : Fin (mem prop)) (t : Nat)
      (e : Multiset ((Nat × Ballot (mem prop)) × Option P)),
      (payloads_to_send i)[t]? = some e ↔
        ∃ (ind : List (Nat × P)) (rcl : List ((Nat × Ballot (mem prop)) × Option P))
          (b : Ballot (mem prop)) (l : Bool),
          (indexed_payloads i)[t]? = some ind ∧ (rc.1 i)[t]? = some rcl
          ∧ (p_ballot i)[t]? = some b ∧ (p_is_leader i)[t]? = some l
          ∧ e = Multiset.ofList
              (if l then ind.map (fun sp => ((sp.1, b), some sp.2)) ++ rcl else []) :=
    fun i t e => by
    refine Iff.trans (hpayloads_to_send_at i t e) ?_
    have hstep : ∀ (ind : List (Nat × P)) (rcl : List ((Nat × Ballot (mem prop)) × Option P))
        (b : Ballot (mem prop)) (l : Bool),
        @id (Multiset ((Nat × Ballot (mem prop)) × Option P))
            (payloads_to_send_step i () (ind, (Multiset.ofList rcl, (b, l)))).2
          = Multiset.ofList
              (if l then ind.map (fun sp => ((sp.1, b), some sp.2)) ++ rcl else []) := by
      intro ind rcl b l
      simp only [id, payloads_to_send_step, den]
      cases l
      · rfl
      · simp only [if_true, Multiset.coe_add, List.map_map]
        rfl
    have hfl : ∀ t' : Nat, (p_log_to_recommit i)[t']? = ((rc.1 i)[t']?).map Multiset.ofList := by
      intro t'
      show ((rc.1 i).map (fun l => Multiset.ofList l))[t']? = _
      rw [List.getElem?_map]
    constructor
    · rintro ⟨ind, m, b, l, hind, hm, hb, hl, rfl⟩
      rw [hfl, Option.map_eq_some_iff] at hm
      obtain ⟨rcl, hrcl, rfl⟩ := hm
      exact ⟨ind, rcl, b, l, hind, hrcl, hb, hl, hstep ind rcl b l⟩
    · rintro ⟨ind, rcl, b, l, hind, hrcl, hb, hl, rfl⟩
      exact ⟨ind, Multiset.ofList rcl, b, l, hind,
        by rw [hfl, hrcl]; rfl, hb, hl, (hstep ind rcl b l).symm⟩
  -- **a tick's send, decoded**: a sent key-value at tick `t` is a leader
  -- tick's, carrying its ballot, and sits in the tick's send list — a
  -- stamped fresh payload or a recommit (`RCTick.owned`)
  ghost have hmem_tick : ∀ (i : Fin (mem prop)) {t : Nat}
      {e : Multiset ((Nat × Ballot (mem prop)) × Option P)} {slot : Nat}
      {b : Ballot (mem prop)} {v : Option P},
      (payloads_to_send i)[t]? = some e → ((slot, b), v) ∈ e →
      ∃ (ind : List (Nat × P)) (rcl : List ((Nat × Ballot (mem prop)) × Option P)),
        (indexed_payloads i)[t]? = some ind ∧ (rc.1 i)[t]? = some rcl
        ∧ (p_ballot i)[t]? = some b ∧ (p_is_leader i)[t]? = some true
        ∧ ((slot, b), v) ∈ ind.map (fun sp => ((sp.1, b), some sp.2)) ++ rcl :=
    fun i {t e slot b v} ht hkv => by
    obtain ⟨ind, rcl, b', l, hind, hrcl, hb', hl, rfl⟩ := (hsend_at i t e).mp ht
    cases l with
    | false => cases hkv
    | true =>
      have hkv' : ((slot, b), v) ∈ ind.map (fun sp => ((sp.1, b'), some sp.2)) ++ rcl :=
        Multiset.mem_coe.mp hkv
      -- the key's ballot is the tick's
      have hbb : b' = b := by
        rcases List.mem_append.mp hkv' with h | h
        · obtain ⟨sp, -, hsp⟩ := List.mem_map.mp h
          exact congrArg (fun x => x.1.2) hsp
        · obtain ⟨gv, b'', m, hg, hb'', hm, hrct⟩ := hrc.commits_at i hrcl
          rw [Trace.read_inj hb' hb'']
          exact (hrct.owned _ h).symm
      subst hbb
      exact ⟨ind, rcl, hind, hrcl, hb', hl, hkv'⟩
  -- **within a tick, keys are distinct**: fresh slots are consecutive and
  -- sit above the gated view's max slot, below which every recommit sits
  ghost have hkeys_nodup : ∀ (i : Fin (mem prop)) {t : Nat} {ind : List (Nat × P)}
      {rcl : List ((Nat × Ballot (mem prop)) × Option P)} {b : Ballot (mem prop)},
      (indexed_payloads i)[t]? = some ind → (rc.1 i)[t]? = some rcl →
      (p_ballot i)[t]? = some b →
      ((ind.map (fun sp => ((sp.1, b), some sp.2)) ++ rcl).map Prod.fst).Nodup :=
    fun i {t ind rcl b} hind hrcl hb => by
    obtain ⟨gv, b', m, hg, hb', hm, hrct⟩ := hrc.commits_at i hrcl
    rw [List.map_append, List.map_map]
    refine List.Nodup.append ?_ ?_ ?_
    · exact List.Nodup.of_map Prod.fst (by
        rw [List.map_map]; exact hip.slots_nodup i hind)
    · exact List.Nodup.of_map Prod.fst (by
        rw [List.map_map]; exact hrct.slots_nodup)
    · intro k hk1 hk2
      obtain ⟨sp, hsp, rfl⟩ := List.mem_map.mp hk1
      obtain ⟨e, he, heq⟩ := List.mem_map.mp hk2
      obtain ⟨mm, hmm, hle⟩ := hrct.slot_le_max e he
      subst hmm
      have hlt := hip.rebase_dominates i (le_refl t) hm (fun w h1 h2 => absurd h1 (by omega))
        hind sp.1 (List.mem_map.mpr ⟨sp, hsp, rfl⟩)
      have hslot : e.1.1 = sp.1 := congrArg Prod.fst heq
      omega
  -- **the reign's single fire** (guarded): the gate fires at the reign's
  -- first leader tick, on the input view (the register cannot name the
  -- ballot: a fire before would be an earlier leader tick of the reign),
  -- and at no later tick of the reign (once per ballot)
  ghost have hreign_fire : ∀ (hreq : LeaderDiscipline (mem prop) P p_ballot p_is_leader p_relevant_p1bs),
      variant = .guarded → ∀ (i : Fin (mem prop)) {t : Nat} {b : Ballot (mem prop)}
      {gt : Multiset (ALog P (mem prop))},
      (p_is_leader i)[t]? = some true → (p_ballot i)[t]? = some b →
      (rcGated i)[t]? = some gt →
      ∃ t₀ v₀, t₀ ≤ t ∧ (p_is_leader i)[t₀]? = some true ∧ (p_ballot i)[t₀]? = some b
        ∧ (p_relevant_p1bs i)[t₀]? = some v₀ ∧ v₀ ≠ 0 ∧ (rcGated i)[t₀]? = some v₀
        ∧ (∀ u, t₀ < u → u ≤ t → (rcGated i)[u]? = some 0)
        ∧ (∀ u, u < t₀ → (p_is_leader i)[u]? = some true → (p_ballot i)[u]? ≠ some b) :=
    fun hreq hvar i {t b gt} hl hb hgt => by
    obtain ⟨t₀, hle, hl₀, hb₀, hd₀, hcontig, hmin⟩ := hreq.reign i hl hb
    have hro : variant.recommitOnce = true := by rw [hvar]; rfl
    -- every tick up to `t` is realized on the gate wire and its inputs
    have hreads : ∀ u, u ≤ t → ∃ (gu vu : Multiset (ALog P (mem prop))) (lu du : Bool),
        (rcGated i)[u]? = some gu
        ∧ (p_relevant_p1bs i)[u]? = some vu ∧ (p_is_leader i)[u]? = some lu
        ∧ (false :: p_is_leader i)[u]? = some du := by
      intro u hu
      obtain ⟨vt, bl_t, dt, hvt, -, hdt, -⟩ := (hgate_at i t gt).mp hgt
      exact ⟨_, _, _, _, List.getElem?_eq_getElem (Nat.lt_of_le_of_lt hu (Trace.read_lt hgt)),
        List.getElem?_eq_getElem (Nat.lt_of_le_of_lt hu (Trace.read_lt hvt)),
        List.getElem?_eq_getElem (Nat.lt_of_le_of_lt hu (Trace.read_lt hl)),
        List.getElem?_eq_getElem (Nat.lt_of_le_of_lt hu (Trace.read_lt hdt))⟩
    obtain ⟨g₀, v₀, l₀, d₀, hg₀, hv₀, hl₀', hd₀'⟩ := hreads t₀ hle
    obtain rfl := Trace.read_inj hl₀ hl₀'
    obtain rfl := Trace.read_inj hd₀ hd₀'
    have hne₀ : v₀ ≠ 0 := hreq.lead_ne i hl₀ hv₀
    -- a fire's input, in the history: its tick is a leader tick carrying
    -- the fire's ballot (reading the gate there)
    have hfire_of : ∀ (u : Nat) (gu : Multiset (ALog P (mem prop))),
        (rcGated i)[u]? = some gu → gu ≠ 0 →
        ∃ vu bu, (p_relevant_p1bs i)[u]? = some vu ∧ (p_ballot i)[u]? = some bu
          ∧ (p_is_leader i)[u]? = some true := by
      intro u gu hgu hne
      obtain ⟨vu, blu, du, hvu, hblu, hdu, -⟩ := (hgate_at i u gu).mp hgu
      have hblu' : (Trace.zip (p_ballot i) (p_is_leader i))[u]? = some blu := hblu
      obtain ⟨hbu, hlu⟩ := Trace.getElem?_zip_eq_some'.mp hblu'
      have hiff := (hfire i hgu hvu hbu hlu hdu).1
      obtain ⟨-, hcond⟩ := hiff.mp hne
      rcases hcond with h | h
      · rw [hro] at h; cases h
      · exact ⟨vu, blu.1, hvu, hbu, by rw [hlu, h.1]⟩
    -- the register before `t₀` does not name `b`: a fire it names would be
    -- an earlier leader tick carrying `b`
    have hra : ra i t₀ ≠ some b := by
      intro hra
      obtain ⟨p, hp, hfp, hpb⟩ := (hgate_inv i t₀ i (fun x hx =>
          hreq.own i x.2.1.1 (List.of_mem_zip (List.of_mem_zip (List.of_mem_zip hx).2).1).1)
        (Trace.pairwise_of_reads (fun x : SPGateIn P (mem prop) => x.2.1.1)
          (fun n x hx => ((Trace.getElem?_zip_eq_some'.mp
            (Trace.getElem?_zip_eq_some'.mp (Trace.getElem?_zip_eq_some'.mp hx).2).1).1))
          (hreq.mono i))).reg_some hro b hra
      obtain ⟨u, hpx, hpg⟩ := Trace.mem_hist_iff.mp hp
      rw [List.getElem?_take] at hpg
      split at hpg
      · obtain ⟨vu, bu, hvu, hbu, hlu⟩ := hfire_of u p.2 hpg hfp
        have hbu' : (p_ballot i)[u]? = some p.1.2.1.1 :=
          (Trace.getElem?_zip_eq_some'.mp (Trace.getElem?_zip_eq_some'.mp
            (Trace.getElem?_zip_eq_some'.mp hpx).2).1).1
        exact hmin u (by assumption) hlu (by rw [hbu', hpb])
      · cases hpg
    have hfire₀ : g₀ = v₀ := by
      obtain ⟨hiff, hpass⟩ := hfire i hg₀ hv₀ hb₀ hl₀ hd₀
      exact hpass (hiff.mpr ⟨hne₀, Or.inr ⟨rfl, rfl, hra⟩⟩)
    subst hfire₀
    refine ⟨t₀, g₀, hle, hl₀, hb₀, hv₀, hne₀, hg₀, ?_, hmin⟩
    -- no later tick of the reign fires: two fires never share a ballot
    intro u hu₀ hut
    obtain ⟨gu, vu, lu, du, hgu, hvu, hlu, hdu⟩ := hreads u hut
    rw [hgu]
    congr 1
    by_contra hne
    have honce := (hgate_inv i (t + 1) i (fun x hx =>
        hreq.own i x.2.1.1 (List.of_mem_zip (List.of_mem_zip (List.of_mem_zip hx).2).1).1)
      (Trace.pairwise_of_reads (fun x : SPGateIn P (mem prop) => x.2.1.1)
        (fun n x hx => ((Trace.getElem?_zip_eq_some'.mp
          (Trace.getElem?_zip_eq_some'.mp (Trace.getElem?_zip_eq_some'.mp hx).2).1).1))
        (hreq.mono i))).once hro
    have hmem : ∀ w gw, w ≤ t → (rcGated i)[w]? = some gw →
        ∀ vw lw dw, (p_relevant_p1bs i)[w]? = some vw → (p_is_leader i)[w]? = some lw →
        (false :: p_is_leader i)[w]? = some dw →
        (Trace.hist (Trace.zip (p_relevant_p1bs i)
          (Trace.zip (Trace.zip (p_ballot i) (p_is_leader i)) (false :: p_is_leader i)))
          ((rcGated i).take (t + 1)))[w]?
          = some ((vw, (((p_ballot i)[w]?.getD b, lw), dw)), gw) := by
      intro w gw hw hgw vw lw dw hvw hlw hdw
      have hbw : (p_ballot i)[w]? = some ((p_ballot i)[w]?.getD b) := by
        obtain ⟨bw, hbw⟩ : ∃ bw, (p_ballot i)[w]? = some bw :=
          ⟨_, List.getElem?_eq_getElem (Nat.lt_of_le_of_lt hw (Trace.read_lt hb))⟩
        rw [hbw]; rfl
      simp only [Trace.hist, List.getElem?_zip_eq_some, List.getElem?_take,
        Nat.lt_succ_of_le hw, if_true]
      exact ⟨⟨hvw, ⟨hbw, hlw⟩, hdw⟩, hgw⟩
    have h₀ := hmem t₀ g₀ hle hg₀ g₀ true false hv₀ hl₀ hd₀
    have hu := hmem u gu hut hgu vu lu du hvu hlu hdu
    have := Trace.pairwise_reads honce hu₀ h₀ hu hne₀ hne
    simp only at this
    rw [hb₀, hcontig u (Nat.le_of_lt hu₀) hut] at this
    exact this rfl
  -- **a sent value is the leader's opened value**: a leader tick's sent
  -- `((slot, b), v)` is characterized against the tick's gated view —
  -- a fresh payload sits above every slot of the view (nothing to
  -- cover), a recommit carries the view's champion (`RCTick.value_best`)
  ghost have hsend_open : ∀ (i : Fin (mem prop)) {t : Nat} {ind : List (Nat × P)}
      {rcl : List ((Nat × Ballot (mem prop)) × Option P)} {slot : Nat}
      {b : Ballot (mem prop)} {v : Option P},
      (indexed_payloads i)[t]? = some ind → (rc.1 i)[t]? = some rcl →
      (p_ballot i)[t]? = some b →
      ((slot, b), v) ∈ ind.map (fun sp => ((sp.1, b), some sp.2)) ++ rcl →
      ∃ gv : Multiset (ALog P (mem prop)), (rcGated i)[t]? = some gv ∧
        ∀ e₀ : LogValue P (mem prop), (slot, e₀) ∈ rcEntries gv →
          ∃ best : LogValue P (mem prop),
            (slot, best) ∈ logView (rcEntries gv) ∧ v = best.value
            ∧ e₀.ballot.ble best.ballot = true :=
    fun i {t ind rcl slot b v} hind hrcl hb hkv => by
    obtain ⟨gv, b', m, hg, hb', hm, hrct⟩ := hrc.commits_at i hrcl
    obtain rfl := Trace.read_inj hb hb'
    refine ⟨gv, hg, fun e₀ he₀ => ?_⟩
    rcases List.mem_append.mp hkv with h | h
    · -- a fresh payload sits strictly above the view's slots: nothing to cover
      exfalso
      obtain ⟨sp, hsp, heq⟩ := List.mem_map.mp h
      obtain ⟨mm, hmm, hslm⟩ := hrct.max_ge slot e₀ he₀
      subst hmm
      have hlt := hip.rebase_dominates i (le_refl t) hm (fun w h1 h2 => absurd h1 (by omega))
        hind sp.1 (List.mem_map.mpr ⟨sp, hsp, rfl⟩)
      have hslot : sp.1 = slot := congrArg (fun x => x.1.1) heq
      omega
    · -- a recommit carries the champion's value
      exact hrct.value_best _ h e₀ he₀
  -- **distinct ticks never share a key** (guarded): two leader ticks
  -- `t < t'` carrying `b` sit in `b`'s reign, which fired once at its
  -- first tick `t₀ ≤ t`; `t'` sends a payload (no recommit after the
  -- fire), and either `t` sends a payload too (slots climb between
  -- rebases) or `t = t₀` recommits (the fire's rebase puts every later
  -- slot above the recommit)
  ghost have hsend_disjoint : ∀ (hreq : LeaderDiscipline (mem prop) P p_ballot p_is_leader p_relevant_p1bs),
      variant = .guarded → ∀ (i : Fin (mem prop)) {t t' : Nat} {slot : Nat}
      {b : Ballot (mem prop)} {va vb : Option P} {ind ind' : List (Nat × P)}
      {rcl rcl' : List ((Nat × Ballot (mem prop)) × Option P)}, t < t' →
      (indexed_payloads i)[t]? = some ind → (rc.1 i)[t]? = some rcl →
      (p_ballot i)[t]? = some b → (p_is_leader i)[t]? = some true →
      ((slot, b), va) ∈ ind.map (fun sp => ((sp.1, b), some sp.2)) ++ rcl →
      (indexed_payloads i)[t']? = some ind' → (rc.1 i)[t']? = some rcl' →
      (p_ballot i)[t']? = some b → (p_is_leader i)[t']? = some true →
      ((slot, b), vb) ∈ ind'.map (fun sp => ((sp.1, b), some sp.2)) ++ rcl' → False :=
    fun hreq hvar i {t t' slot b va vb ind ind' rcl rcl'} hlt_ hind hrcl hbt hlt hkv
      hind' hrcl' hbt' hlt' hkv' => by
    -- the reign of `b`, from the later tick
    obtain ⟨gv', b'', m', hg', hb'', hm', hrct'⟩ := hrc.commits_at i hrcl'
    obtain rfl := Trace.read_inj hbt' hb''
    obtain ⟨t₀, v₀, hle, hl₀, hb₀, hv₀, hne₀, hg₀, hclosed, hmin⟩ :=
      hreign_fire hreq hvar i hlt' hbt' hg'
    -- `t` is in the reign too: a leader tick carrying `b` is not before
    -- the reign's first tick
    have ht₀t : t₀ ≤ t := by
      by_contra hlt0
      exact hmin t (Nat.lt_of_not_le hlt0) hlt hbt
    -- after the fire, no tick of the reign rebases (an empty gated view
    -- recommits nothing and has no max slot)
    have hnone : ∀ w, t₀ < w → w ≤ t' → (rc.2 i)[w]? = some none := by
      intro w hw₀ hwt
      obtain ⟨bw, hbw⟩ : ∃ bw, (p_ballot i)[w]? = some bw :=
        ⟨_, List.getElem?_eq_getElem (Nat.lt_of_le_of_lt hwt (Trace.read_lt hbt'))⟩
      obtain ⟨rclw, mw, -, hmw, hrctw⟩ := hrc.tick_at i (hclosed w hw₀ hwt) hbw
      rw [hmw, (hrctw.view_empty rfl).2]
    -- the later tick sends a payload (its gated view is empty)
    have hgv' : gv' = 0 :=
      (Option.some.inj ((hclosed t' (Nat.lt_of_le_of_lt ht₀t hlt_) (le_refl _)).symm.trans hg')).symm
    subst hgv'
    rw [(hrct'.view_empty rfl).1, List.append_nil] at hkv'
    obtain ⟨sp', hsp', heq'⟩ := List.mem_map.mp hkv'
    have hslot' : sp'.1 = slot := congrArg (fun x => x.1.1) heq'
    rcases List.mem_append.mp hkv with h | h
    · -- payload at `t`: the register climbed strictly between `t` and `t'`
      obtain ⟨sp, hsp, heq⟩ := List.mem_map.mp h
      have hslot : sp.1 = slot := congrArg (fun x => x.1.1) heq
      have := hip.slots_dominate i hlt_
        (fun w hw hw' => hnone w (Nat.lt_of_le_of_lt ht₀t hw) hw')
        hind hind' sp.1 (List.mem_map.mpr ⟨sp, hsp, rfl⟩) sp'.1 (List.mem_map.mpr ⟨sp', hsp', rfl⟩)
      omega
    · -- recommit at `t`: only the fire tick recommits, and its max slot
      -- sits below every later slot
      obtain ⟨gv, b', m, hg, hb', hm, hrct⟩ := hrc.commits_at i hrcl
      obtain rfl := Trace.read_inj hbt hb'
      have ht₀ : t = t₀ := by
        by_contra hne
        have hgv : gv = 0 := (Option.some.inj
          ((hclosed t (Nat.lt_of_le_of_ne ht₀t (Ne.symm hne)) (Nat.le_of_lt hlt_)).symm.trans hg)).symm
        subst hgv
        rw [(hrct.view_empty rfl).1] at h
        cases h
      subst ht₀
      obtain rfl := Trace.read_inj hg hg₀
      obtain ⟨mm, hmm, hle_m⟩ := hrct.slot_le_max _ h
      subst hmm
      have := hip.rebase_dominates i (Nat.le_of_lt hlt_) hm (fun w hw hw' => hnone w hw hw')
        hind' sp'.1 (List.mem_map.mpr ⟨sp', hsp', rfl⟩)
      change slot ≤ mm at hle_m
      omega
  -- **a key is sent at most once** (guarded): within a tick keys are
  -- distinct; across ticks a key never recurs
  ghost have hkey_once : ∀ (hreq : LeaderDiscipline (mem prop) P p_ballot p_is_leader p_relevant_p1bs),
      variant = .guarded → ∀ (i : Fin (mem prop)) (k : Nat × Ballot (mem prop)),
      ((payloads_to_send i).sum).countP (fun x => x.1 = k) ≤ 1 :=
    fun hreq hvar i k => by
    refine Trace.sum_countP_le_one _ (payloads_to_send i) ?_ ?_
    · intro t e ht
      obtain ⟨ind, rcl, b, l, hind, hrcl, hb, hl, rfl⟩ := (hsend_at i t e).mp ht
      cases l with
      | false => simp
      | true => exact countP_key_le_one (hkeys_nodup i hind hrcl hb) k
    · intro t t' e e' htt ht ht' x hx y hy hxk hyk
      obtain ⟨⟨s₁, b₁⟩, v₁⟩ := x
      obtain ⟨⟨s₂, b₂⟩, v₂⟩ := y
      simp only at hxk hyk
      subst hxk
      obtain ⟨rfl, rfl⟩ := Prod.mk.inj hyk
      obtain ⟨ind, rcl, hind, hrcl, hb, hl, hkv⟩ := hmem_tick i ht hx
      obtain ⟨ind', rcl', hind', hrcl', hb', hl', hkv'⟩ := hmem_tick i ht' hy
      exact hsend_disjoint hreq hvar i htt hind hrcl hb hl hkv hind' hrcl' hb' hl' hkv'
  -- **two sends at one key agree** (guarded): the same tick's keys are
  -- distinct, and no key recurs across ticks
  ghost have hsend_once : ∀ (hreq : LeaderDiscipline (mem prop) P p_ballot p_is_leader p_relevant_p1bs),
      variant = .guarded → ∀ (i : Fin (mem prop)) {t t' : Nat}
      {e e' : Multiset ((Nat × Ballot (mem prop)) × Option P)} {slot : Nat}
      {b : Ballot (mem prop)} {va vb : Option P},
      (payloads_to_send i)[t]? = some e → ((slot, b), va) ∈ e →
      (payloads_to_send i)[t']? = some e' → ((slot, b), vb) ∈ e' → va = vb :=
    fun hreq hvar i {t t' e e' slot b va vb} ht ha ht' hb => by
    obtain ⟨ind, rcl, hind, hrcl, hbt, hlt, hkv⟩ := hmem_tick i ht ha
    obtain ⟨ind', rcl', hind', hrcl', hbt', hlt', hkv'⟩ := hmem_tick i ht' hb
    rcases Nat.lt_trichotomy t t' with h | rfl | h
    · exact (hsend_disjoint hreq hvar i h hind hrcl hbt hlt hkv hind' hrcl' hbt' hlt' hkv').elim
    · -- the same tick: its keys are distinct
      obtain rfl := Trace.read_inj hind hind'
      obtain rfl := Trace.read_inj hrcl hrcl'
      exact congrArg Prod.snd (nodup_keys_inj (hkeys_nodup i hind hrcl hbt) hkv hkv' rfl)
    · exact (hsend_disjoint hreq hvar i h hind' hrcl' hbt' hlt' hkv' hind hrcl hbt hlt hkv).elim
  -- **a sent value is opened against the INPUT view** (guarded): the
  -- tick's input view is the fire tick's (same reign, both leading:
  -- `pinned`), which the gate passed through — so the gated view's
  -- characterization is the input view's: `LeaderOpen`
  ghost have hsend_opened : ∀ (hreq : LeaderDiscipline (mem prop) P p_ballot p_is_leader p_relevant_p1bs),
      variant = .guarded → ∀ (i : Fin (mem prop)) {t : Nat}
      {e : Multiset ((Nat × Ballot (mem prop)) × Option P)} {slot : Nat}
      {b : Ballot (mem prop)} {v : Option P},
      (payloads_to_send i)[t]? = some e → ((slot, b), v) ∈ e →
      LeaderOpen (p_ballot i) (p_is_leader i) (p_relevant_p1bs i) b slot v :=
    fun hreq hvar i {t e slot b v} ht hkv => by
    obtain ⟨ind, rcl, hind, hrcl, hbt, hlt, hkv'⟩ := hmem_tick i ht hkv
    obtain ⟨gv, hg, hopen⟩ := hsend_open i hind hrcl hbt hkv'
    obtain ⟨t₀, v₀, hle, hl₀, hb₀, hv₀, hne₀, hg₀, hclosed, -⟩ :=
      hreign_fire hreq hvar i hlt hbt hg
    -- the tick's input view is the fire tick's
    obtain ⟨vt, hvt⟩ : ∃ vt, (p_relevant_p1bs i)[t]? = some vt :=
      ⟨_, List.getElem?_eq_getElem (by
        obtain ⟨_, _, _, hvt', _, _, _⟩ := (hgate_at i t gv).mp hg
        exact Trace.read_lt hvt')⟩
    have hview : vt = v₀ := hreq.pinned i hlt hl₀ hbt hb₀ rfl hvt hv₀
    subst hview
    refine ⟨t, vt, hlt, hbt, hvt, fun e₀ he₀ => ?_⟩
    -- a recommit happens only at the fire tick, on the input view; a
    -- payload's slot is above the reign's recovered max slot
    obtain ⟨gv', b', m, hg', hb', hm, hrct⟩ := hrc.commits_at i hrcl
    obtain rfl := Trace.read_inj hbt hb'
    obtain rfl := Trace.read_inj hg hg'
    rcases Nat.lt_or_eq_of_le hle with hlt₀ | rfl
    · -- after the fire: the gated view is empty, so the send is a payload,
      -- above the fire tick's max slot (no rebase since)
      have hgv : gv = 0 :=
        (Option.some.inj ((hclosed t hlt₀ (le_refl _)).symm.trans hg)).symm
      subst hgv
      exfalso
      rw [(hrct.view_empty rfl).1, List.append_nil] at hkv'
      obtain ⟨sp, hsp, heq⟩ := List.mem_map.mp hkv'
      have hslot : sp.1 = slot := congrArg (fun x => x.1.1) heq
      have hnone : ∀ w, t₀ < w → w ≤ t → (rc.2 i)[w]? = some none := by
        intro w hw₀ hwt
        obtain ⟨bw, hbw⟩ : ∃ bw, (p_ballot i)[w]? = some bw :=
          ⟨_, List.getElem?_eq_getElem (Nat.lt_of_le_of_lt hwt (Trace.read_lt hbt))⟩
        obtain ⟨rclw, mw, -, hmw, hrctw⟩ := hrc.tick_at i (hclosed w hw₀ hwt) hbw
        rw [hmw, (hrctw.view_empty rfl).2]
      obtain ⟨rcl₀, m₀, -, hm₀, hrct₀⟩ := hrc.tick_at i hg₀ hb₀
      obtain ⟨mm, hmm, hslm⟩ := hrct₀.max_ge slot e₀ he₀
      subst hmm
      have := hip.rebase_dominates i (Nat.le_of_lt hlt₀) hm₀ hnone hind sp.1
        (List.mem_map.mpr ⟨sp, hsp, rfl⟩)
      omega
    · -- the fire tick: the gated view IS the input view
      obtain rfl := Trace.read_inj hg' hg₀
      exact hopen e₀ he₀
  -- payloads_to_send.clone().end_atomic()
  --   .map(q!(move |((slot, ballot), value)| P2a { sender: CLUSTER_SELF_ID, ballot, slot, value }))
  --   .broadcast(acceptors, TCP…).values()
  let p2as := H.values (H.broadcast_closed sched.p2aCh
    (H.map (H.allTicks payloads_to_send)
      (fun me kv => (⟨me, kv.1.2, kv.1.1, kv.2⟩ : P2a P (mem prop)))))
  -- the P2a pool face, at the acceptor boundary: per-sender streams,
  -- summed (values ∘ broadcast_closed), each the sender's send wire
  ghost have hp2apool : ∀ (j : Fin (mem acc)),
      p2as j = ((List.finRange (mem prop)).map
        (fun r => ((payloads_to_send r).sum).map
          (fun kv => (⟨r, kv.1.2, kv.1.1, kv.2⟩ : P2a P (mem prop))))).sum :=
    fun j => by simp only [p2as, den]
  -- a pool P2a is some tick's send of its sender
  ghost have hp2a_src : ∀ (j : Fin (mem acc)) (p2a : P2a P (mem prop)), p2a ∈ p2as j →
      ∃ (r : Fin (mem prop)) (t : Nat) (e : Multiset ((Nat × Ballot (mem prop)) × Option P)),
        p2a.sender = r ∧ (payloads_to_send r)[t]? = some e
        ∧ ((p2a.slot, p2a.ballot), p2a.value) ∈ e :=
    fun j p2a hp => by
    rw [hp2apool j] at hp
    obtain ⟨ms, hms, hmem⟩ := mem_list_sum.mp hp
    obtain ⟨r, -, rfl⟩ := List.mem_map.mp hms
    obtain ⟨kv, hkv, rfl⟩ := Multiset.mem_map.mp hmem
    obtain ⟨e, he, hkve⟩ := mem_list_sum.mp hkv
    obtain ⟨t, ht⟩ := List.mem_iff_getElem?.mp he
    exact ⟨r, t, e, rfl, ht, hkve⟩
  -- a pool P2a's ballot is its sender's (under the input requirement)
  ghost have hp2a_own : ∀ (hreq : LeaderDiscipline (mem prop) P p_ballot p_is_leader p_relevant_p1bs)
      (j : Fin (mem acc)) (p2a : P2a P (mem prop)), p2a ∈ p2as j →
      p2a.ballot.proposerId = p2a.sender := fun hreq j p2a hp => by
    obtain ⟨r, t, e, hr, ht, hkv⟩ := hp2a_src j p2a hp
    obtain ⟨-, -, -, -, hbt, -, -⟩ := hmem_tick r ht hkv
    rw [hr]
    exact hreq.own r _ (Trace.mem_of_read hbt)
  -- **pool P2as at one key agree** (guarded): both are their owner's sends
  -- at the key, and the owner sends a key once
  ghost have hp2a_agree : ∀ (hreq : LeaderDiscipline (mem prop) P p_ballot p_is_leader p_relevant_p1bs),
      variant = .guarded → ∀ (j j' : Fin (mem acc)) (p q : P2a P (mem prop)),
      p ∈ p2as j → q ∈ p2as j' → p.slot = q.slot → p.ballot = q.ballot → p.value = q.value :=
    fun hreq hvar j j' p q hp hq hs hb => by
    obtain ⟨r, t, e, hr, ht, hkv⟩ := hp2a_src j p hp
    obtain ⟨r', t', e', hr', ht', hkv'⟩ := hp2a_src j' q hq
    have hrr : r = r' := by
      rw [← hr, ← hr', ← hp2a_own hreq j p hp, ← hp2a_own hreq j' q hq, hb]
    subst hrr
    rw [hs, hb] at hkv
    exact hsend_once hreq hvar r ht hkv ht' hkv'
  -- acceptor_p2(a_max_ballot, p2as, a_checkpoint)
  let ap2 := acceptor_p2 H acc prop a_max_ballot p2as a_checkpoint
    dec.ap2 sched.ap2
  -- the sub-module contract, at the acceptor wires
  ghost have hap2 := acceptor_p2.ensures acc prop a_max_ballot p2as a_checkpoint
    dec.ap2 sched.ap2
  -- **a published entry is a pool P2a's** (guarded): it quotes a consumed
  -- P2a at its key and, the pool's P2as at the key agreeing, carries its
  -- value — some sender's send
  ghost have hlog_entry_src : ∀ (hreq : LeaderDiscipline (mem prop) P p_ballot p_is_leader p_relevant_p1bs),
      variant = .guarded → ∀ (j : Fin (mem acc)) {t : Nat} {lg : ALog P (mem prop)}
      {slot : Nat} {e : LogValue P (mem prop)},
      (ap2.1 j)[t]? = some lg → (slot, e) ∈ lg.2 →
      ∃ (p2a : P2a P (mem prop)), p2a ∈ p2as j ∧ p2a.slot = slot ∧ p2a.ballot = e.ballot
        ∧ p2a.value = e.value :=
    fun hreq hvar j {t lg slot e} hlg hen => by
    obtain ⟨⟨p2a, hp, hs, hb⟩, hval⟩ := hap2.log_entry_src j hlg hen
    refine ⟨p2a, hp, hs, hb, ?_⟩
    refine (hval p2a.value fun q hq hqs hqb => ?_).symm
    exact hp2a_agree hreq hvar j j q p2a hq hp (hqs.trans hs.symm) (hqb.trans hb.symm)
  -- collect_quorum(a_to_proposers_p2b, f + 1, 2 * f + 1)
  let p2b_pairs := H.map ap2.2
    (fun _me m => ((m.slot, m.ballot), m.res))
  let cq := collect_quorum H prop p2b_pairs (f + 1) (2 * f + 1)
    dec.cqBatch
  ghost have hcq := collect_quorum.ensures prop p2b_pairs (f + 1) (2 * f + 1)
    dec.cqBatch
  -- p_to_replicas = join_responses(
  --   quorums.map(q!(|key| (key, ()))),
  --   payloads_to_send…all_ticks_atomic() [atomic: not a decision])
  let jr_responses := H.map cq.1 (fun _me k => (k, ()))
  let jr := join_responses H prop jr_responses
    payloads_to_send dec.jrBatch
  ghost have hjr := join_responses.ensures prop jr_responses
    payloads_to_send dec.jrBatch
  -- **a commit opens through the stages** (guarded): the joined metadata is
  -- a send of this proposer at the key; the key was quorum'd by `f + 1`
  -- `Ok` votes among the acks — `f + 1` distinct acceptors (per-acceptor
  -- unit caps: an acceptor's votes at the key are at most its pool's P2as
  -- there, at most one by send-once at the owner), each with a covering
  -- log at its vote tick
  ghost have hcommit : ∀ (hreq : LeaderDiscipline (mem prop) P p_ballot p_is_leader p_relevant_p1bs),
      variant = .guarded → ∀ {i : Fin (mem prop)} {slot : Nat} {v : Option P},
      (slot, v) ∈ (jr i).map (fun kmv => (kmv.1.1, kmv.2.1)) →
      ∃ (b : Ballot (mem prop)) (t : Nat) (e : Multiset ((Nat × Ballot (mem prop)) × Option P)),
        (payloads_to_send i)[t]? = some e ∧ ((slot, b), v) ∈ e
        ∧ SPChosen f (fun j => a_checkpoint j (dec.ap2.ckSnap j)) a_max_ballot ap2.1 slot b :=
    fun hreq hvar {i slot v} hmem => by
    obtain ⟨kmv, hkmv, hproj⟩ := Multiset.mem_map.mp hmem
    obtain ⟨k, m, u⟩ := kmv
    obtain ⟨hmd, hresp⟩ := hjr.join_src i hkmv
    have hslot : slot = k.1 := (congrArg Prod.fst hproj).symm
    have hv_eq : v = m := (congrArg Prod.snd hproj).symm
    subst hslot hv_eq
    -- the joined metadata is this proposer's send at the key
    obtain ⟨e, he, hke⟩ := mem_list_sum.mp hmd
    obtain ⟨t, ht⟩ := List.mem_iff_getElem?.mp he
    refine ⟨k.2, t, e, ht, hke, ?_⟩
    -- the joined key was emitted by `collect_quorum`, hence it holds
    -- `f + 1` `Ok` votes among the consumed acks (`emit_sound`) — and
    -- a fortiori among the full ack pool
    have hk_q : k ∈ cq.1 i := by
      have h1 : (k, u) ∈ (cq.1 i).map (fun k0 => (k0, ())) :=
        Multiset.mem_of_le (cqConsumed_le _ _) hresp
      obtain ⟨k0, hk0, hkeq⟩ := Multiset.mem_map.mp h1
      rw [← show k0 = k from congrArg Prod.fst hkeq]
      exact hk0
    have hsound : f + 1 ≤ cqOkCount (cqConsumed
        ((ap2.2 i).map (fun m => ((m.slot, m.ballot), m.res)))
        (dec.cqBatch i)) k := hcq.emit_sound i k hk_q
    have hqle : f < ((ap2.2 i).filter
        (fun m => m.slot = k.1 ∧ m.ballot = k.2
          ∧ m.res = .ok ())).card := by
      have hchain : cqOkCount (cqConsumed
          ((ap2.2 i).map (fun m => ((m.slot, m.ballot), m.res)))
          (dec.cqBatch i)) k
          ≤ cqOkCount ((ap2.2 i).map
            (fun m => ((m.slot, m.ballot), m.res))) k :=
        cqOkCount_mono (cqConsumed_le _ _) k
      have hbr : cqOkCount ((ap2.2 i).map (fun m => ((m.slot, m.ballot), m.res))) k
          = ((ap2.2 i).filter (fun m => m.slot = k.1 ∧ m.ballot = k.2
              ∧ m.res = .ok ())).card := by
        unfold cqOkCount
        rw [Multiset.filter_map, Multiset.card_map]
        congr 1
        refine Multiset.filter_congr fun m _ => ?_
        simp only [Function.comp]
        constructor
        · rintro ⟨hk, hok⟩
          refine ⟨congrArg Prod.fst hk, congrArg Prod.snd hk, ?_⟩
          cases hres : m.res with
          | ok u => cases u; rfl
          | error e => rw [hres] at hok; cases hok
        · rintro ⟨h1, h2, h3⟩
          exact ⟨by rw [h1, h2, Prod.mk.eta], by rw [h3]; rfl⟩
      omega
    -- the ok votes, by acceptor (the acceptor's face)
    obtain ⟨byAcc, hdec, hcap, hsrc⟩ := hap2.acks_by_acceptor i
    have hvotes : (ap2.2 i).filter
        (fun m => m.slot = k.1 ∧ m.ballot = k.2 ∧ m.res = .ok ())
        = ((List.finRange (mem acc)).map
          (fun j => (byAcc j).filter
            (fun m => m.slot = k.1 ∧ m.ballot = k.2 ∧ m.res = .ok ()))).sum := by
      rw [hdec, filter_list_sum, List.map_map]
      rfl
    -- per-acceptor unit caps
    have hown_b : (k.2).proposerId = i := by
      obtain ⟨-, -, -, -, hbt, -, -⟩ := hmem_tick i ht hke
      exact hreq.own i _ (Trace.mem_of_read hbt)
    have hcaps : ∀ j ∈ List.finRange (mem acc),
        ((byAcc j).filter
          (fun m => m.slot = k.1 ∧ m.ballot = k.2 ∧ m.res = .ok ())).card ≤ 1 := by
      intro j _
      have h1 : ((byAcc j).filter
          (fun m => m.slot = k.1 ∧ m.ballot = k.2 ∧ m.res = .ok ())).card
          ≤ ((byAcc j).filter (fun m => m.slot = k.1 ∧ m.ballot = k.2)).card := by
        rw [← Multiset.countP_eq_card_filter, ← Multiset.countP_eq_card_filter]
        exact countP_impl_le _ _ _ (fun m hm => ⟨hm.1, hm.2.1⟩)
      refine le_trans h1 (le_trans (hcap j k.1 k.2) ?_)
      rw [hp2apool j, countP_list_sum, List.map_map]
      refine sum_map_le_single (List.nodup_finRange _) _ i ?_ ?_
      · -- other senders never carry `i`'s ballot
        intro r _ hri
        simp only [Function.comp]
        rw [Multiset.countP_eq_zero]
        intro p2a hp hpk
        have hown' := hp2a_own hreq j p2a (by
          rw [hp2apool j]
          exact mem_list_sum.mpr ⟨_, List.mem_map.mpr ⟨r, List.mem_finRange r, rfl⟩, hp⟩)
        obtain ⟨kv, -, rfl⟩ := Multiset.mem_map.mp hp
        have hb2 : kv.1.2 = k.2 := hpk.2
        have hown'' : kv.1.2.proposerId = r := hown'
        rw [hb2, hown_b] at hown''
        exact hri hown''.symm
      · -- the owner sends the key at most once (B2 + freshness)
        simp only [Function.comp]
        refine le_trans (countP_map_le_countP _ _
          (fun kv => kv.1 = k)
          (fun kv hkv => by
            cases kv with
            | mk kk vv =>
              cases kk with
              | mk s0 b0 =>
                have h1 : s0 = k.1 := hkv.1
                have h2 : b0 = k.2 := hkv.2
                show (s0, b0) = k
                rw [h1, h2]) _) ?_
        exact hkey_once hreq hvar i k
    -- distinct acceptors from the unit caps
    have hle2 : (ap2.2 i).filter
        (fun m => m.slot = k.1 ∧ m.ballot = k.2 ∧ m.res = .ok ())
        ≤ ((List.finRange (mem acc)).map
          (fun j => (byAcc j).filter
            (fun m => m.slot = k.1 ∧ m.ballot = k.2 ∧ m.res = .ok ()))).sum :=
      le_of_eq hvotes
    obtain ⟨C, hCnd, hCsub, hCcard, hCrep⟩ := exists_distinct_reps
      (List.finRange (mem acc)) _ _ (List.nodup_finRange _) hle2 hcaps
    refine ⟨C, hCnd, le_trans hqle hCcard, ?_⟩
    intro j hj
    obtain ⟨x, hxv, hxq⟩ := hCrep j hj
    have hxpred := Multiset.of_mem_filter hxq
    have hxfrom : x ∈ byAcc j := Multiset.mem_of_le (Multiset.filter_le _ _) hxq
    obtain ⟨p2a, -, -, -, -, hcov⟩ := hsrc j x hxfrom
    obtain ⟨t', hmx, hlog⟩ := hcov hxpred.2.2
    obtain ⟨hta, hmx'⟩ := List.getElem?_eq_some_iff.mp hmx
    refine ⟨t', hta, by rw [hmx', hxpred.2.1], fun hck => ?_⟩
    obtain ⟨lt, hlt, hcov'⟩ := hlog hck
    obtain ⟨htl, hlt'⟩ := List.getElem?_eq_some_iff.mp hlt
    refine ⟨htl, ?_⟩
    show LogCovers ((ap2.1 j)[t']'htl).2 k.1 k.2
    rw [hlt', ← hxpred.2.1, ← hxpred.1]
    exact hcov'
  -- **the witness of a committed value** (guarded): its send's ballot —
  -- owned, chosen (`hcommit`), opened by the leader (`hsend_opened`), and
  -- agreed by every published entry at the key (an entry is a pool P2a's,
  -- a pool P2a is a send, sends at one key agree)
  ghost have hwitness : ∀ (hreq : LeaderDiscipline (mem prop) P p_ballot p_is_leader p_relevant_p1bs),
      variant = .guarded → ∀ {i : Fin (mem prop)} {slot : Nat} {v : Option P}
      {b : Ballot (mem prop)} {t : Nat} {e : Multiset ((Nat × Ballot (mem prop)) × Option P)},
      (payloads_to_send i)[t]? = some e → ((slot, b), v) ∈ e →
      SPChosen f (fun j => a_checkpoint j (dec.ap2.ckSnap j)) a_max_ballot ap2.1 slot b →
      CommitWitness f p_ballot p_is_leader p_relevant_p1bs
        (fun j => a_checkpoint j (dec.ap2.ckSnap j)) a_max_ballot ap2.1 i slot v b :=
    fun hreq hvar {i slot v b t e} ht hkv hch =>
    { owned := by
        obtain ⟨-, -, -, -, hbt, -, -⟩ := hmem_tick i ht hkv
        exact hreq.own i _ (Trace.mem_of_read hbt)
      chosen := hch
      opened := hsend_opened hreq hvar i ht hkv
      entries_agree := fun j {t' lg e'} hlg hen hb' => by
        obtain ⟨p2a, hp, hs, hpb, hpv⟩ := hlog_entry_src hreq hvar j hlg hen
        obtain ⟨r, t'', e'', hr, ht'', hkv''⟩ := hp2a_src j p2a hp
        have hri : r = i := by
          rw [← hr, ← hp2a_own hreq j p2a hp, hpb, hb']
          obtain ⟨-, -, -, -, hbt, -, -⟩ := hmem_tick i ht hkv
          exact hreq.own i _ (Trace.mem_of_read hbt)
        subst hri
        rw [hs, hpb, hb'] at hkv''
        rw [← hpv]
        exact (hsend_once hreq hvar r ht'' hkv'' ht hkv) }
  -- fails.flat_map_ordered(q!(|(_, ballot)| ballot))
  let fail_ballots := H.filterMap cq.2 (fun _me ke => ke.2)
  -- .map(q!(|((slot, _ballot), (value, _))| (slot, value)))
  (H.map jr (fun _me kmv => (kmv.1.1, kmv.2.1)),
    ap2.1, fail_ballots)
  prove
    log_entry_open := (fun hreq hvar j {t lg slot e} hlg hen => by
      obtain ⟨p2a, hp, hs, hb, hv⟩ := hlog_entry_src hreq hvar j hlg hen
      obtain ⟨r, t', e', hr, ht', hkv'⟩ := hp2a_src j p2a hp
      have hri : r = e.ballot.proposerId := by rw [← hb, ← hr, hp2a_own hreq j p2a hp]
      subst hri
      rw [hs, hb, hv] at hkv'
      exact hsend_opened hreq hvar _ ht' hkv'),
    log_entry_agree := (fun hreq hvar j j' {t t' lg lg' slot e e'} hlg hen hlg' hen' hbb => by
      obtain ⟨p, hp, hps, hpb, hpv⟩ := hlog_entry_src hreq hvar j hlg hen
      obtain ⟨q, hq, hqs, hqb, hqv⟩ := hlog_entry_src hreq hvar j' hlg' hen'
      rw [← hpv, ← hqv]
      exact hp2a_agree hreq hvar j j' p q hp hq (hps.trans hqs.symm) (by rw [hpb, hqb, hbb])),
    commit_spec := (fun hreq hvar {i slot v} hmem => by
      subst hvar
      obtain ⟨b, t, e, ht, hkv, hch⟩ := hcommit hreq rfl hmem
      exact ⟨b, hwitness hreq rfl ht hkv hch⟩),
    commit_distinct := (fun hreq hvar {i i' slot v v'} hmem hmem' hne => by
      subst hvar
      obtain ⟨b, t, e, ht, hkv, hch⟩ := hcommit hreq rfl hmem
      obtain ⟨b', t', e', ht', hkv', hch'⟩ := hcommit hreq rfl hmem'
      refine ⟨b, b', fun hbb => hne ?_, hwitness hreq rfl ht hkv hch,
        hwitness hreq rfl ht' hkv' hch'⟩
      -- one ballot, one owner: both sends are the owner's at one key
      subst hbb
      have hii : i = i' := by
        rw [← (hwitness hreq rfl ht hkv hch).owned, ← (hwitness hreq rfl ht' hkv' hch').owned]
      subst hii
      exact hsend_once hreq rfl i ht hkv ht' hkv'),
    log_len_le_ck := hap2.log_len_le_ck,
    log_covers_mono := (fun {j t t'} h htl' {slot b} hcov =>
      hap2.log_covers_mono j h (List.getElem?_eq_getElem (Nat.lt_of_le_of_lt h htl'))
        (List.getElem?_eq_getElem htl') hcov)

/-! ## Executable smoke test: one proposer, one acceptor, f = 0 —
a payload flows through sequencing, acceptance, quorum, and lands at
the replica. -/

#guard (sequence_payload (Values PaxLoc (paxMem 1 1)) .guarded .prop .acc
    (ckα := Nat) (ckord := .totalOrder) (ckret := .exactlyOnce)
    (fun _ => [42])                                   -- client payload
    (fun _ d => d.map (fun _ => none))                -- a_checkpoint
    (fun _ => [Ballot.mk 0 0, Ballot.mk 0 0])         -- p_ballot
    (fun _ => [true, true])                           -- p_is_leader
    (fun _ => [{}, {}])                               -- p_relevant_p1bs
    0
    (fun _ => [some (Ballot.mk 0 0), some (Ballot.mk 0 0)])  -- a_max
    ⟨fun _ => [1, 0],                                 -- payload batch
     ⟨fun _ => [{⟨0, Ballot.mk 0 0, 0, some 42⟩}, {}],  -- p2a batches
      fun _ => [0, 0]⟩,                               -- checkpoint snap
     fun _ => [{((0, Ballot.mk 0 0), .ok ())}, {}],    -- cq acks
     fun _ => [{((0, Ballot.mk 0 0), ())}, {}]⟩        -- jr key batches
    .triv                                             -- sched-det bundle
    ).1 0
  = ({(0, some 42)} : Multiset (Nat × Option Nat))

#nondet_census sequence_payload (nondets := 5) (scheds := 2) (fuels := 0)

end Hydro
