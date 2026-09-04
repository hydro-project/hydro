import HydroV2.Paxos.SequencePayloadLemmas
import HydroV2.Std.Quorum
import HydroV2.Std.RequestResponse
import HydroV2.HydroDef

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
callees** — the commit calculus below is over this module's own pure
tick functions, and `acceptor_p2`'s behavior enters only through its
contract (`AP2Ensures`).
-/

namespace HydroV2

variable {L : Type} {mem : L → Nat} {P : Type} [DecidableEq P]

/-- `sequence_payload`'s `nondet!` sites (paxos.rs:596–734), incl. the
P2a consumption it hands to `acceptor_p2`. -/
structure SPDec (H : HydroSem L mem) (nP nA : Nat) (P : Type)
    [DecidableEq P] where
  /-- `c_to_proposers.batch(&proposer_tick, nondet_commit)`: payload
  slice sizes. -/
  payloadBatch : H.OrdBatchDec nP
  /-- The recommit gate's per-tick view emission
  (`emitMultisetBatches`). -/
  rcEmit : H.EmitDec nP (ALog P nP)
  /-- `p_to_acceptors_p2a.broadcast(&acceptors, …)`: P2a transport
  (proposers → acceptors). -/
  p2aCh : H.TransportDec nA nP
  /-- `p_to_acceptors_p2a.batch(&acceptor_tick, nondet!(…))`
  (`acceptor_p2`'s consumption): consumed P2a increments. -/
  p2aBatch : H.BatchDec nA (P2a P nP)
  /-- `a_to_proposers_p2b.demux(&proposers, …)`: P2b transport
  (acceptors → proposers). -/
  p2bCh : H.TransportDec nP nA
  /-- `collect_quorum`'s `use::batch(responses, nondet!(…))`
  (quorum.rs:97) on the keyed P2b acks: consumed ack increments. -/
  cqBatch : H.BatchDec nP
    ((Nat × Ballot nP) × Except (Option (Ballot nP)) Unit)
  /-- `collect_quorum`'s just-reached-key emission
  (`emitMultisetBatches`). -/
  cqEmit : H.EmitDec nP (Nat × Ballot nP)
  /-- `join_responses`'s `use::batch(responses, nondet!(…))`
  (request_response.rs:23) on the freshly quorum'd keys. The metadata
  side is `atomic` (paxos.rs:764–770) — same-tick availability, not a
  decision. -/
  jrBatch : H.BatchDec nP ((Nat × Ballot nP) × Unit)
  /-- `join_responses`'s joined-pair emission
  (`emitMultisetBatches`). -/
  jrEmit : H.EmitDec nP ((Nat × Ballot nP) × (Option P × Unit))

/-- Repack `Values`-typed decisions for the `MonoRel` instantiations
(the two instances declare the same decision vocabulary). -/
private def spdecMR {nP nA : Nat} {P : Type} [DecidableEq P]
    (d : SPDec (Values L mem) nP nA P) :
    SPDec (MonoRel L mem) nP nA P :=
  ⟨d.payloadBatch, d.rcEmit, d.p2aCh, d.p2aBatch, d.p2bCh,
   d.cqBatch, d.cqEmit, d.jrBatch, d.jrEmit⟩


/-! ## Local helpers (the acceptor-side assembly's small calculus) -/

/-- What `sequence_payload` **ensures**, over the `Values` denotation,
against its inputs. The emission-side calculus (`spSentTrace_open`,
`spSentTrace_key_nodup`, functionality) is pure and lives with the
vocabulary; these fields carry what involves the acceptor half: every
replica commit is an owned emission at a **chosen** key, and every
published log entry quotes an emission (write-before-ack provenance).
Preconditions are per-field (`SPRequires` = `leader_election`'s
guarantees, projected); the faithful variant's fields claim nothing. -/
structure SPEnsures (variant : PaxosVariant) (prop acc : L) (f : Nat)
    (cp : Fin (mem prop) → List P)
    (ck : Fin (mem acc) → Trace (Option Nat))
    (pb : Fin (mem prop) → Trace (Ballot (mem prop)))
    (pl : Fin (mem prop) → Trace Bool)
    (p1bs : Fin (mem prop) → Trace (Multiset (ALog P (mem prop))))
    (mx : Fin (mem acc) → Trace (Option (Ballot (mem prop))))
    (dec : SPDec (Values L mem) (mem prop) (mem acc) P)
    (out : (Fin (mem prop) → Multiset (Nat × Option P))
      × (Fin (mem acc) → Trace (ALog P (mem prop)))
      × (Fin (mem prop) → Multiset (Ballot (mem prop)))) : Prop where
  /-- **Commit** (guarded): every emitted `(slot, value)` of
  `p_to_replicas` is an owned emission at a **chosen** key. -/
  commit_spec : SPRequires (mem prop) P pb pl p1bs →
    variant = .guarded → ∀ {i : Fin (mem prop)} {slot : Nat}
    {v : Option P}, (slot, v) ∈ out.1 i →
    ∃ b : Ballot (mem prop), b.proposerId = i
      ∧ SPEmission .guarded f (cp i) (dec.payloadBatch i) (pb i)
          (pl i) (p1bs i) slot b v
      ∧ SPChosen f ck mx out.2.1 slot b
  /-- **The published clock**: the log wire ticks at most as often as
  the checkpoint input (its tick zip). -/
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
  /-- **Log entry** (guarded): every entry of the published `a_log`
  output quotes an owned emission. -/
  log_entry : SPRequires (mem prop) P pb pl p1bs →
    variant = .guarded → ∀ {j : Fin (mem acc)} {t : Nat}
    (htl : t < (out.2.1 j).length) {slot : Nat}
    {e : LogValue P (mem prop)},
    (slot, e) ∈ ((out.2.1 j)[t]'htl).2 →
    SPEmission .guarded f (cp e.ballot.proposerId)
      (dec.payloadBatch e.ballot.proposerId)
      (pb e.ballot.proposerId) (pl e.ballot.proposerId)
      (p1bs e.ballot.proposerId) slot e.ballot e.value

set_option maxHeartbeats 1000000 in
/-- **paxos.rs:678–774 `sequence_payload`** over proposers `prop` and
acceptors `acc`. Returns (`p_to_replicas`, `a_log`,
`fail_ballots`). -/
hydro def sequence_payload (H : HydroSem L mem) (variant : PaxosVariant)
    (prop acc : L)
    (c_to_proposers : H.Stream prop P .totalOrder .exactlyOnce)
    (a_checkpoint : H.TickSingleton acc (Option Nat) .unbounded)
    (p_ballot : H.TickSingleton prop (Ballot (mem prop)) .unbounded)
    (p_is_leader : H.TickSingleton prop Bool .unbounded)
    (p_relevant_p1bs :
      H.TickStream prop (ALog P (mem prop)) .noOrder .exactlyOnce)
    (f : Nat)
    (a_max_ballot :
      H.TickSingleton acc (Option (Ballot (mem prop))) .unbounded)
    (dec : SPDec H (mem prop) (mem acc) P) :
    (H.Stream prop (Nat × Option P) .noOrder .exactlyOnce
      × H.TickSingleton acc (ALog P (mem prop)) .unbounded
      × H.Stream prop (Ballot (mem prop)) .noOrder .exactlyOnce)
  ensures out => SPEnsures variant prop acc f c_to_proposers
    a_checkpoint p_ballot p_is_leader p_relevant_p1bs a_max_ballot
    dec out :=
  -- GUARDED (B2 fix): recommit (and rebase) once per ballot, on
  -- becoming leader — the accepted view is gated to empty otherwise.
  -- FAITHFUL: the gate is the identity (fires at every nonempty view).
  let rcGated := H.scan_batches_unordered_across_ticks p_relevant_p1bs
    (H.zipTick (H.zipTick p_ballot p_is_leader)
      (H.defer false p_is_leader))
    (fun _me => spGateStep variant.recommitOnce)
    none
  -- recommit_after_leader_election(p_relevant_p1bs, p_ballot, f)
  let rcS := recommit_after_leader_election H prop
    (H.emitMultisetBatches rcGated dec.rcEmit) p_ballot f
  let rc := rcS.val
  ghost have hrc := rcS.property rfl
  -- index_payloads(p_max_slot,
  --   c_to_proposers.batch(tick, nondet_commit).filter_if(p_is_leader))
  let indexed := (index_payloads H prop rc.2
    (H.emitBatches (H.mapBatchWith
      (H.batch_ordered c_to_proposers dec.payloadBatch) p_is_leader
      (fun _me b flag => if flag then b else [])))).val
  -- .cross_singleton(p_ballot).map(((slot, ballot), Some(payload)))
  --   .chain(p_log_to_recommit).filter_if(p_is_leader)
  let payloads_to_send := H.mapTick
    (H.zipTick (H.zipTick indexed rc.1) (H.zipTick p_ballot p_is_leader))
    (fun _me x =>
      if x.2.2 then
        x.1.1.map (fun sp => ((sp.1, x.2.1), some sp.2)) ++ x.1.2
      else [])
  -- the sent face, at the send wire: the pipeline IS the pure
  -- vocabulary (the recommit legs through the wire-chain faces,
  -- everything else pinned by rfl)
  ghost have hsent : ∀ (i : Fin (mem prop)),
      payloads_to_send i = spSentTrace variant f (c_to_proposers i)
        (dec.payloadBatch i) (p_ballot i) (p_is_leader i)
        (p_relevant_p1bs i) := fun i => by
    show (Trace.zip (Trace.zip
        (scanAcrossTicksTrace (fun s bt => ipStep s bt.1 bt.2) 0
          (Trace.zip ((Trace.zip
              (sliceCuts (c_to_proposers i) 0 (dec.payloadBatch i))
              (p_is_leader i)).map
              (fun bx => if bx.2 then bx.1 else []))
            (rc.2 i)))
        (rc.1 i))
      (Trace.zip (p_ballot i) (p_is_leader i))).map
      (fun x =>
        if x.2.2 then
          x.1.1.map (fun sp => ((sp.1, x.2.1), some sp.2)) ++ x.1.2
        else []) = _
    rw [hrc.commits_eq i, hrc.maxslot_eq i]
    rfl
  -- .map(P2a { sender: CLUSTER_SELF_ID, ballot, slot, value })
  --   .broadcast(acceptors, …).values()
  let p2as := H.values (H.broadcast dec.p2aCh (H.allTicks (H.emitBatches
    (H.mapTick payloads_to_send (fun me l =>
      l.map (fun kv =>
        (⟨me, kv.1.2, kv.1.1, kv.2⟩ : P2a P (mem prop))))))))
  -- the P2a pool face, at the acceptor boundary: per-sender streams,
  -- summed (values ∘ broadcast)
  ghost have hp2apool : ∀ (j : Fin (mem acc)),
      p2as j = ((List.finRange (mem prop)).map
        (fun r => (↑(((payloads_to_send r).map (fun l =>
          l.map (fun kv => (⟨r, kv.1.2, kv.1.1, kv.2⟩
            : P2a P (mem prop))))).flatten)
          : Multiset (P2a P (mem prop))))).sum := fun j => rfl
  -- a consumed P2a opens to its sender's emission
  ghost have hp2a_stream_open : ∀ (r : Fin (mem prop))
      (p2a : P2a P (mem prop)),
      p2a ∈ (↑(((payloads_to_send r).map (fun l =>
        l.map (fun kv => (⟨r, kv.1.2, kv.1.1, kv.2⟩
          : P2a P (mem prop))))).flatten)
        : Multiset (P2a P (mem prop))) →
      p2a.sender = r ∧ ((p2a.slot, p2a.ballot), p2a.value)
        ∈ (spSentTrace variant f (c_to_proposers r)
          (dec.payloadBatch r) (p_ballot r) (p_is_leader r)
          (p_relevant_p1bs r)).flatten := fun r p2a hp => by
    have hp' := Multiset.mem_coe.mp hp
    obtain ⟨ch, hch, hpch⟩ := List.mem_flatten.mp hp'
    obtain ⟨l0, hl0, rfl⟩ := List.mem_map.mp hch
    obtain ⟨kv, hkv, rfl⟩ := List.mem_map.mp hpch
    have hkvf : kv ∈ (payloads_to_send r).flatten :=
      List.mem_flatten.mpr ⟨l0, hl0, hkv⟩
    rw [hsent r] at hkvf
    exact ⟨rfl, hkvf⟩
  -- ballots on sender `r`'s stream are owned by `r` (under the input
  -- requirement) — `commit_spec` and `log_entry` both consume it
  ghost have hp2a_own : ∀ (hreq : SPRequires (mem prop) P p_ballot
        p_is_leader p_relevant_p1bs)
      (r : Fin (mem prop)) (p2a : P2a P (mem prop)),
      p2a ∈ (↑(((payloads_to_send r).map (fun l =>
        l.map (fun kv => (⟨r, kv.1.2, kv.1.1, kv.2⟩
          : P2a P (mem prop))))).flatten)
        : Multiset (P2a P (mem prop))) →
      p2a.ballot.proposerId = r := fun hreq r p2a hp => by
    obtain ⟨-, hem⟩ := hp2a_stream_open r p2a hp
    obtain ⟨t0, hpl0, hpb0, -, -, hpbeq, -⟩ :=
      spSentTrace_open variant f (c_to_proposers r)
        (dec.payloadBatch r) (p_ballot r) (p_is_leader r)
        (p_relevant_p1bs r) hem
    rw [← hpbeq]
    exact hreq.own r _ (List.getElem_mem hpb0)
  -- acceptor_p2(a_max_ballot, p2as, a_checkpoint)
  let ap2S := acceptor_p2 H acc prop a_max_ballot p2as a_checkpoint
    dec.p2aBatch dec.p2bCh
  let ap2 := ap2S.val
  -- the sub-module contract + its log face, at the acceptor wires
  ghost have hap2 := ap2S.property rfl
  ghost have hlogface : ∀ (j : Fin (mem acc)),
      ap2.1 j = (Trace.zip (a_checkpoint j)
        (foldAcrossTicksTrace (· + ·) 0
          (ap2QualBatches (mem prop) (a_max_ballot j) (p2as j)
            (dec.p2aBatch j)))).map
        (fun ce => (ce.1, logView ce.2)) := fun j => hap2.log_face j
  -- collect_quorum(a_to_proposers_p2b, f + 1, 2 * f + 1)
  let p2b_pairs := H.map ap2.2
    (fun _me m => ((m.slot, m.ballot), m.res))
  let cqS := collect_quorum H prop p2b_pairs (f + 1) (2 * f + 1)
    dec.cqBatch dec.cqEmit
  -- p_to_replicas = join_responses(
  --   quorums.map(q!(|key| (key, ()))),
  --   payloads_to_send…all_ticks_atomic() [atomic: not a decision])
  let jr_responses := H.map cqS.val.1 (fun _me k => (k, ()))
  let jrS := join_responses H prop jr_responses
    (H.emitBatchesUnordered payloads_to_send) dec.jrBatch dec.jrEmit
  -- fails.flat_map_ordered(q!(|(_, ballot)| ballot))
  let fail_ballots := H.filterMap cqS.val.2 (fun _me ke => ke.2)
  -- .map(q!(|((slot, _ballot), (value, _))| (slot, value)))
  (H.map jrS.val (fun _me kmv => (kmv.1.1, kmv.2.1)),
    ap2.1, fail_ballots)
  prove
    commit_spec := (fun hreq hvar {i slot v} hmem => by
      subst hvar
      -- open the joined output through the shared-stage contracts:
      -- `join_responses` quotes its key's metadata and consumed
      -- response; `collect_quorum` certifies the key's votes
      have hmem2 : (slot, v) ∈ (jrS.val i).map
          (fun kmv => (kmv.1.1, kmv.2.1)) := hmem
      obtain ⟨kmv, hkmv, hproj⟩ := Multiset.mem_map.mp hmem2
      obtain ⟨k, m, u⟩ := kmv
      have hjr := jrS.property rfl
      obtain ⟨hmd, hresp⟩ := hjr.join_src i hkmv
      have hslot : slot = k.1 := (congrArg Prod.fst hproj).symm
      have hv_eq : v = m := (congrArg Prod.snd hproj).symm
      -- the joined metadata is a realized emission
      have he_em : ((slot, k.2), v)
          ∈ (spSentTrace .guarded f (c_to_proposers i)
            (dec.payloadBatch i) (p_ballot i) (p_is_leader i)
            (p_relevant_p1bs i)).flatten := by
        rw [← hsent i]
        have h0 : (k, m) ∈ (payloads_to_send i).flatten :=
          mem_ofList_sum.mp hmd
        have hkm : ((slot, k.2), v) = (k, m) := by
          rw [hslot, hv_eq, Prod.mk.eta]
        rw [hkm]
        exact h0
      -- ownership of the key's ballot
      have hown_b : (k.2).proposerId = i := by
        obtain ⟨t0, hpl0, hpb0, -, -, hpbeq, -⟩ :=
          spSentTrace_open .guarded f (c_to_proposers i)
            (dec.payloadBatch i) (p_ballot i) (p_is_leader i)
            (p_relevant_p1bs i) he_em
        rw [← hpbeq]
        exact hreq.own i _ (List.getElem_mem hpb0)
      refine ⟨k.2, hown_b, he_em, ?_⟩
      -- the joined key was emitted by `collect_quorum`, hence it holds
      -- `f + 1` `Ok` votes among the consumed acks (`emit_sound`) — and
      -- a fortiori among the full ack pool
      have hcq := cqS.property rfl
      have hk_q : k ∈ cqS.val.1 i := by
        have h1 : (k, u) ∈ (cqS.val.1 i).map (fun k0 => (k0, ())) :=
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
        have hbr := cqOkCount_p2b_pairs (ap2.2 i) k
        omega
      -- decompose the ok votes per sender
      have hdec : ap2.2 i = ((List.finRange (mem acc)).map
          (fun j => ap2From (fun j' => batchCuts (p2as j') 0
            (dec.p2aBatch j')) (fun j' => a_max_ballot j') j i)).sum :=
        hap2.ack_decomp i
      have hvotes : (ap2.2 i).filter
          (fun m => m.slot = k.1 ∧ m.ballot = k.2 ∧ m.res = .ok ())
          = ((List.finRange (mem acc)).map
            (fun j => (ap2From (fun j' => batchCuts (p2as j') 0
              (dec.p2aBatch j')) (fun j' => a_max_ballot j') j i).filter
              (fun m => m.slot = k.1 ∧ m.ballot = k.2
                ∧ m.res = .ok ()))).sum := by
        rw [hdec, filter_list_sum, List.map_map]
        rfl
      -- per-acceptor unit caps (send-once at the key)
      have hcaps : ∀ j ∈ List.finRange (mem acc),
          ((ap2From (fun j' => batchCuts (p2as j') 0
            (dec.p2aBatch j')) (fun j' => a_max_ballot j') j i).filter
            (fun m => m.slot = k.1 ∧ m.ballot = k.2
              ∧ m.res = .ok ())).card ≤ 1 := by
        intro j _
        have h1 : ((ap2From (fun j' => batchCuts (p2as j') 0
            (dec.p2aBatch j')) (fun j' => a_max_ballot j') j i).filter
            (fun m => m.slot = k.1 ∧ m.ballot = k.2
              ∧ m.res = .ok ())).card
            ≤ ((ap2From (fun j' => batchCuts (p2as j') 0
              (dec.p2aBatch j')) (fun j' => a_max_ballot j') j i).filter
              (fun m => m.slot = k.1 ∧ m.ballot = k.2)).card := by
          rw [← Multiset.countP_eq_card_filter,
            ← Multiset.countP_eq_card_filter]
          exact countP_impl_le _ _ _ (fun m hm => ⟨hm.1, hm.2.1⟩)
        refine le_trans h1 ?_
        refine le_trans (hap2.from_key_cap j i k.1 k.2) ?_
        have h2 : ((batchCuts (p2as j) 0 (dec.p2aBatch j)).sum).countP
            (fun a => a.slot = k.1 ∧ a.ballot = k.2)
            ≤ (p2as j).countP
              (fun a => a.slot = k.1 ∧ a.ballot = k.2) := by
          refine Multiset.countP_le_of_le _ ?_
          have := batchCuts_sum_le (pool := p2as j)
            (d := dec.p2aBatch j) (consumed := 0) (Multiset.zero_le _)
          rwa [Multiset.zero_add] at this
        refine le_trans h2 ?_
        rw [hp2apool j, countP_list_sum, List.map_map]
        refine sum_map_le_single (List.nodup_finRange _) _ i ?_ ?_
        · -- other senders never carry `i`'s ballot
          intro r _ hri
          simp only [Function.comp]
          rw [Multiset.countP_eq_zero]
          intro p2a hp hpk
          have hballi : p2a.ballot.proposerId = i := by
            rw [hpk.2, hown_b]
          have hballr : p2a.ballot.proposerId = r := hp2a_own hreq r p2a hp
          rw [hballi] at hballr
          exact hri hballr.symm
        · -- the owner sends the key at most once (B2 + freshness)
          simp only [Function.comp]
          have hml : (↑(((payloads_to_send i).map (fun l =>
              l.map (fun kv => (⟨i, kv.1.2, kv.1.1, kv.2⟩
                : P2a P (mem prop))))).flatten)
              : Multiset (P2a P (mem prop)))
              = (↑((payloads_to_send i).flatten)
                : Multiset ((Nat × Ballot (mem prop)) × Option P)).map
                (fun kv => (⟨i, kv.1.2, kv.1.1, kv.2⟩
                  : P2a P (mem prop))) := by
            rw [← List.map_flatten]
            rfl
          rw [hml]
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
          have hnd : (((payloads_to_send i).flatten).map
              Prod.fst).Nodup := by
            rw [hsent i]
            exact spSentTrace_key_nodup f (c_to_proposers i)
              (dec.payloadBatch i) i (p_ballot i) (p_is_leader i)
              (p_relevant_p1bs i) (hreq.own i)
              (fun h ht' => hreq.mono i h ht')
              (fun ht1 hb1 h1 h0 => hreq.stable i ht1 hb1 h1 h0)
              (fun hpl hpr hl => hreq.lead_ne i hpl hpr hl)
          exact countP_key_le_one hnd k
      -- distinct acceptors from the unit caps
      have hle2 : (ap2.2 i).filter
          (fun m => m.slot = k.1 ∧ m.ballot = k.2 ∧ m.res = .ok ())
          ≤ ((List.finRange (mem acc)).map
            (fun j => (ap2From (fun j' => batchCuts (p2as j') 0
              (dec.p2aBatch j')) (fun j' => a_max_ballot j') j i).filter
              (fun m => m.slot = k.1 ∧ m.ballot = k.2
                ∧ m.res = .ok ()))).sum := le_of_eq hvotes
      obtain ⟨C, hCnd, hCsub, hCcard, hCrep⟩ := exists_distinct_reps
        (List.finRange (mem acc)) _ _ (List.nodup_finRange _) hle2 hcaps
      refine ⟨C, hCnd, ?_, ?_⟩
      · refine le_trans ?_ hCcard
        exact hqle
      · intro j hj
        obtain ⟨x, hxv, hxq⟩ := hCrep j hj
        have hxpred := Multiset.of_mem_filter hxq
        have hxfrom : x ∈ ap2From (fun j' => batchCuts (p2as j') 0
            (dec.p2aBatch j')) (fun j' => a_max_ballot j') j i :=
          Multiset.mem_of_le (Multiset.filter_le _ _) hxq
        obtain ⟨t, hb, hm, p2a, hp2ain, hrt, hslot2, hball2, hshape,
          hcov⟩ := hap2.from_src j i x hxfrom
        obtain ⟨hmx, hlc⟩ := hcov hxpred.2.2
        refine ⟨t, hm, ?_, ?_⟩
        · rw [hmx, hxpred.2.1]
        · intro hck
          -- the published log realizes the vote tick: its length is the
          -- min of the checkpoint, the consumed batches and the max wire
          have htl2 : t < (ap2.1 j).length := by
            rw [hlogface j]
            simp only [List.length_map, Trace.zip, List.length_zip,
              foldAcrossTicksTrace_length]
            unfold ap2QualBatches
            simp only [List.length_map, Trace.zip, List.length_zip]
            omega
          refine ⟨htl2, ?_⟩
          have := hlc htl2
          rw [hxpred.1, hxpred.2.1] at this
          rw [hslot]
          exact this
),
    log_len_le_ck := (fun j => by
      rw [hlogface j]
      simp only [List.length_map, Trace.zip, List.length_zip]
      omega
),
    log_covers_mono := (fun {j t t'} h htl' {slot b} hcov => by
      have hzl' : t' < (Trace.zip (a_checkpoint j)
          (foldAcrossTicksTrace (· + ·) 0
            (ap2QualBatches (mem prop) (a_max_ballot j) (p2as j)
              (dec.p2aBatch j)))).length := by
        have h0 := htl'
        rw [hlogface j, List.length_map] at h0
        exact h0
      have hfl' : t' < (foldAcrossTicksTrace (· + ·) 0
          (ap2QualBatches (mem prop) (a_max_ballot j) (p2as j)
            (dec.p2aBatch j))).length := by
        have h0 := hzl'
        simp only [Trace.zip, List.length_zip] at h0
        omega
      have hck' : t' < (a_checkpoint j).length := by
        have h0 := hzl'
        simp only [Trace.zip, List.length_zip] at h0
        omega
      -- both ticks' published values, opened through the face
      have hopen : ∀ (u : Nat) (hu : u < (ap2.1 j).length)
          (hfu : u < (foldAcrossTicksTrace (· + ·) 0
            (ap2QualBatches (mem prop) (a_max_ballot j) (p2as j)
              (dec.p2aBatch j))).length),
          ((ap2.1 j)[u]'hu).2 = logView
            ((foldAcrossTicksTrace (· + ·) 0
              (ap2QualBatches (mem prop) (a_max_ballot j) (p2as j)
                (dec.p2aBatch j)))[u]'hfu) := by
        intro u hu hfu
        have hzu : u < (Trace.zip (a_checkpoint j)
            (foldAcrossTicksTrace (· + ·) 0
              (ap2QualBatches (mem prop) (a_max_ballot j) (p2as j)
                (dec.p2aBatch j)))).length := by
          have h0 := hu
          rw [hlogface j, List.length_map] at h0
          exact h0
        have hcku : u < (a_checkpoint j).length := by
          have h0 := hzu
          simp only [Trace.zip, List.length_zip] at h0
          omega
        rw [List.getElem_of_eq (hlogface j) hu, List.getElem_map]
        have hpair : (Trace.zip (a_checkpoint j)
            (foldAcrossTicksTrace (· + ·) 0
              (ap2QualBatches (mem prop) (a_max_ballot j) (p2as j)
                (dec.p2aBatch j))))[u]'hzu
            = ((a_checkpoint j)[u]'hcku,
               (foldAcrossTicksTrace (· + ·) 0
                (ap2QualBatches (mem prop) (a_max_ballot j) (p2as j)
                  (dec.p2aBatch j)))[u]'hfu) := by
          simp only [Trace.zip]
          exact List.getElem_zip ..
        rw [hpair]
      have hflt : t < (foldAcrossTicksTrace (· + ·) 0
          (ap2QualBatches (mem prop) (a_max_ballot j) (p2as j)
            (dec.p2aBatch j))).length := Nat.lt_of_le_of_lt h hfl'
      rw [hopen t (Nat.lt_of_le_of_lt h htl') hflt] at hcov
      rw [hopen t' htl' hfl']
      -- the accumulated entries grow with the tick
      refine logView_covers_mono ?_ hcov
      rw [foldAcrossTicksTrace_getElem, foldAcrossTicksTrace_getElem,
        ← List.sum_eq_foldl, ← List.sum_eq_foldl]
      have hpre : ((ap2QualBatches (mem prop) (a_max_ballot j) (p2as j)
          (dec.p2aBatch j)).take (t + 1))
          <+: ((ap2QualBatches (mem prop) (a_max_ballot j) (p2as j)
            (dec.p2aBatch j)).take (t' + 1)) := by
        rw [show (ap2QualBatches (mem prop) (a_max_ballot j) (p2as j)
            (dec.p2aBatch j)).take (t + 1)
          = ((ap2QualBatches (mem prop) (a_max_ballot j) (p2as j)
            (dec.p2aBatch j)).take (t' + 1)).take (t + 1) by
          rw [List.take_take, Nat.min_eq_left (by omega)]]
        exact List.take_prefix _ _
      exact sublist_sum_le hpre.sublist
),
    log_entry := (fun hreq hvar {j t} htl {slot e} hentry => by
      subst hvar
      -- open the published log through `acceptor_p2`'s face
      have hzl : t < (Trace.zip (a_checkpoint j)
          (foldAcrossTicksTrace (· + ·) 0
            (ap2QualBatches (mem prop) (a_max_ballot j) (p2as j)
              (dec.p2aBatch j)))).length := by
        have h0 := htl
        rw [hlogface j, List.length_map] at h0
        exact h0
      have hfl : t < (foldAcrossTicksTrace (· + ·) 0
          (ap2QualBatches (mem prop) (a_max_ballot j) (p2as j)
            (dec.p2aBatch j))).length := by
        have h0 := hzl
        simp only [Trace.zip, List.length_zip] at h0
        omega
      have hentry2 : (slot, e) ∈ logView
          ((foldAcrossTicksTrace (· + ·) 0
            (ap2QualBatches (mem prop) (a_max_ballot j) (p2as j)
              (dec.p2aBatch j)))[t]'hfl) := by
        have h0 := hentry
        rw [List.getElem_of_eq (hlogface j) htl, List.getElem_map] at h0
        have hpair : (Trace.zip (a_checkpoint j)
            (foldAcrossTicksTrace (· + ·) 0
              (ap2QualBatches (mem prop) (a_max_ballot j) (p2as j)
                (dec.p2aBatch j))))[t]'hzl
            = ((a_checkpoint j)[t]'(by
                have h1 := hzl
                simp only [Trace.zip, List.length_zip] at h1
                omega),
               (foldAcrossTicksTrace (· + ·) 0
                (ap2QualBatches (mem prop) (a_max_ballot j) (p2as j)
                  (dec.p2aBatch j)))[t]'hfl) := by
          simp only [Trace.zip]
          exact List.getElem_zip ..
        rw [hpair] at h0
        exact h0
      -- the accumulated entries through tick `t`
      have hacc : (foldAcrossTicksTrace (· + ·) 0
          (ap2QualBatches (mem prop) (a_max_ballot j) (p2as j)
            (dec.p2aBatch j)))[t]'hfl
          = ((ap2QualBatches (mem prop) (a_max_ballot j) (p2as j)
            (dec.p2aBatch j)).take (t + 1)).sum := by
        rw [foldAcrossTicksTrace_getElem, List.sum_eq_foldl]
      -- every accumulated entry at `(slot, e.ballot)` quotes an owned
      -- emission at that key
      have htrace : ∀ lv : LogValue P (mem prop),
          (slot, lv) ∈ (foldAcrossTicksTrace (· + ·) 0
            (ap2QualBatches (mem prop) (a_max_ballot j) (p2as j)
              (dec.p2aBatch j)))[t]'hfl →
          lv.ballot = e.ballot →
          ((slot, e.ballot), lv.value)
            ∈ (spSentTrace .guarded f
              (c_to_proposers e.ballot.proposerId)
              (dec.payloadBatch e.ballot.proposerId)
              (p_ballot e.ballot.proposerId)
              (p_is_leader e.ballot.proposerId)
              (p_relevant_p1bs e.ballot.proposerId)).flatten := by
        intro lv hlv hlvb
        rw [hacc] at hlv
        obtain ⟨chunk, hchunk, hlvin⟩ := mem_list_sum.mp hlv
        have hchunk2 : chunk ∈ ap2QualBatches (mem prop)
            (a_max_ballot j) (p2as j) (dec.p2aBatch j) :=
          (List.take_sublist ..).subset hchunk
        unfold ap2QualBatches at hchunk2
        obtain ⟨bx, hbx, rfl⟩ := List.mem_map.mp hchunk2
        obtain ⟨p2a, hp2a_in, hqual⟩ :=
          (Multiset.mem_filterMap _ _).mp hlvin
        by_cases hq : p2aQualifies bx.2 p2a.ballot
        · rw [if_pos hq] at hqual
          injection hqual with hqual'
          have hps : p2a.slot = slot := congrArg Prod.fst hqual'
          have hpe : (⟨p2a.ballot, p2a.value⟩ : LogValue P (mem prop))
              = lv := congrArg Prod.snd hqual'
          have hpb2 : p2a.ballot = e.ballot := by
            rw [← hlvb, ← hpe]
          have hpv : p2a.value = lv.value := by
            rw [← hpe]
          -- the consumed P2a came from the merged pool
          have hin_pool : p2a ∈ p2as j := by
            have hb1 : bx.1 ∈ batchCuts (p2as j) 0 (dec.p2aBatch j) :=
              (List.of_mem_zip hbx).1
            have hmem_sum : p2a ∈ (batchCuts (p2as j) 0
                (dec.p2aBatch j)).sum :=
              mem_list_sum.mpr ⟨bx.1, hb1, hp2a_in⟩
            have hle := batchCuts_sum_le (pool := p2as j)
              (d := dec.p2aBatch j) (consumed := 0) (Multiset.zero_le _)
            rw [Multiset.zero_add] at hle
            exact Multiset.mem_of_le hle hmem_sum
          -- pool → some sender's stream → an emission of that sender
          rw [hp2apool j] at hin_pool
          obtain ⟨ms, hms, hmem⟩ := mem_list_sum.mp hin_pool
          obtain ⟨r, -, rfl⟩ := List.mem_map.mp hms
          obtain ⟨-, hem⟩ := hp2a_stream_open r p2a hmem
          have hro : r = e.ballot.proposerId := by
            rw [← hp2a_own hreq r p2a hmem, hpb2]
          subst hro
          rw [hps, hpb2, hpv] at hem
          exact hem
        · rw [if_neg hq] at hqual
          cases hqual
      -- the canonical entry's value is the group's agreed value
      obtain ⟨hval, lv0, hlv0, hlv0b⟩ := logView_entry _ hentry2
      have hem0 := htrace lv0 hlv0 hlv0b
      -- keys are sent once: all group values agree with `lv0.value`
      have hnd : (((spSentTrace .guarded f
          (c_to_proposers e.ballot.proposerId)
          (dec.payloadBatch e.ballot.proposerId)
          (p_ballot e.ballot.proposerId)
          (p_is_leader e.ballot.proposerId)
          (p_relevant_p1bs e.ballot.proposerId)).flatten).map
          Prod.fst).Nodup :=
        spSentTrace_key_nodup f _ _ e.ballot.proposerId _ _ _
          (hreq.own _)
          (fun h ht' => hreq.mono _ h ht')
          (fun ht1 hb1 h1 h0 => hreq.stable _ ht1 hb1 h1 h0)
          (fun hpl hpr hl => hreq.lead_ne _ hpl hpr hl)
      have hgroup : ∀ w ∈ ((((foldAcrossTicksTrace (· + ·) 0
          (ap2QualBatches (mem prop) (a_max_ballot j) (p2as j)
            (dec.p2aBatch j)))[t]'hfl).filter
            (fun x : Nat × LogValue P (mem prop) => x.1 = slot)).map
            Prod.snd).filter
            (fun lv : LogValue P (mem prop) => lv.ballot = e.ballot),
          w.value = lv0.value := by
        intro w hw
        have hw1 := Multiset.of_mem_filter hw
        have hw2 : w ∈ (((foldAcrossTicksTrace (· + ·) 0
            (ap2QualBatches (mem prop) (a_max_ballot j) (p2as j)
              (dec.p2aBatch j)))[t]'hfl).filter
              (fun x : Nat × LogValue P (mem prop) => x.1 = slot)).map
              Prod.snd :=
          Multiset.mem_of_le (Multiset.filter_le _ _) hw
        obtain ⟨x, hx, rfl⟩ := Multiset.mem_map.mp hw2
        have hx1 : x.1 = slot := (Multiset.mem_filter.mp hx).2
        have hx2 : x ∈ (foldAcrossTicksTrace (· + ·) 0
            (ap2QualBatches (mem prop) (a_max_ballot j) (p2as j)
              (dec.p2aBatch j)))[t]'hfl :=
          Multiset.mem_of_le (Multiset.filter_le _ _) hx
        have hxs : (slot, x.2) ∈ (foldAcrossTicksTrace (· + ·) 0
            (ap2QualBatches (mem prop) (a_max_ballot j) (p2as j)
              (dec.p2aBatch j)))[t]'hfl := by
          rw [show ((slot, x.2) : Nat × LogValue P (mem prop))
            = x by rw [← hx1]] at *
          exact hx2
        have hemw := htrace x.2 hxs hw1
        have := nodup_keys_inj hnd hemw hem0 rfl
        exact congrArg Prod.snd this
      -- `valOf` of the agreeing group is that value
      have hne : ((((foldAcrossTicksTrace (· + ·) 0
          (ap2QualBatches (mem prop) (a_max_ballot j) (p2as j)
            (dec.p2aBatch j)))[t]'hfl).filter
            (fun x : Nat × LogValue P (mem prop) => x.1 = slot)).map
            Prod.snd).filter
            (fun lv : LogValue P (mem prop) => lv.ballot = e.ballot)
            ≠ 0 := by
        intro habs
        have hlv0g : lv0 ∈ ((((foldAcrossTicksTrace (· + ·) 0
            (ap2QualBatches (mem prop) (a_max_ballot j) (p2as j)
              (dec.p2aBatch j)))[t]'hfl).filter
              (fun x : Nat × LogValue P (mem prop) => x.1 = slot)).map
              Prod.snd).filter
              (fun lv : LogValue P (mem prop) =>
                lv.ballot = e.ballot) := by
          refine Multiset.mem_filter.mpr ⟨?_, hlv0b⟩
          refine Multiset.mem_map.mpr ⟨(slot, lv0), ?_, rfl⟩
          exact Multiset.mem_filter.mpr ⟨hlv0, rfl⟩
        rw [habs] at hlv0g
        cases hlv0g
      have hvals : e.value = lv0.value := by
        rw [hval]
        refine valOf_const _ ?_ ?_
        · intro habs
          exact hne (Multiset.map_eq_zero.mp habs)
        · intro x hx
          obtain ⟨w, hw, rfl⟩ := Multiset.mem_map.mp hx
          exact hgroup w hw
      show ((slot, e.ballot), e.value) ∈ _
      rw [hvals]
      exact hem0)



/-- Flo monotonicity, generically: instantiate at `MonoRel` (all three
legs at their graded orders). -/
theorem sequence_payload_mono (variant : PaxosVariant)
    (prop acc : L)
    (cp cp' : Fin (mem prop) → List P)
    (ck ck' : TickV (mem acc) (Option Nat) .unbounded)
    (pb pb' : TickV (mem prop) (Ballot (mem prop)) .unbounded)
    (pl pl' : TickV (mem prop) Bool .unbounded)
    (p1bs p1bs' : Fin (mem prop) → Trace (Multiset (ALog P (mem prop))))
    (f : Nat)
    (mx mx' : TickV (mem acc) (Option (Ballot (mem prop))) .unbounded)
    (dec : SPDec (Values L mem) (mem prop) (mem acc) P)
    (hcp : ∀ i, cp i <+: cp' i) (hck : ∀ i, ck i <+: ck' i)
    (hpb : ∀ i, pb i <+: pb' i) (hpl : ∀ i, pl i <+: pl' i)
    (hp1bs : ∀ i, p1bs i <+: p1bs' i) (hmx : ∀ i, mx i <+: mx' i) :
    (∀ i, (sequence_payload (Values L mem) variant prop acc cp ck pb pl p1bs f
        mx dec).val.1 i
      ≤ (sequence_payload (Values L mem) variant prop acc cp' ck' pb' pl' p1bs'
        f mx' dec).val.1 i)
    ∧ (∀ i, (sequence_payload (Values L mem) variant prop acc cp ck pb pl p1bs f
        mx dec).val.2.1 i
      <+: (sequence_payload (Values L mem) variant prop acc cp' ck' pb' pl'
        p1bs' f mx' dec).val.2.1 i)
    ∧ (∀ i, (sequence_payload (Values L mem) variant prop acc cp ck pb pl p1bs f
        mx dec).val.2.2 i
      ≤ (sequence_payload (Values L mem) variant prop acc cp' ck' pb' pl' p1bs'
        f mx' dec).val.2.2 i) :=
  ⟨(sequence_payload (MonoRel L mem) variant prop acc ⟨(cp, cp'), hcp⟩
      ⟨(ck, ck'), hck⟩ ⟨(pb, pb'), hpb⟩ ⟨(pl, pl'), hpl⟩
      ⟨(p1bs, p1bs'), hp1bs⟩ f ⟨(mx, mx'), hmx⟩ (spdecMR dec)).val.1.property,
   (sequence_payload (MonoRel L mem) variant prop acc ⟨(cp, cp'), hcp⟩
      ⟨(ck, ck'), hck⟩ ⟨(pb, pb'), hpb⟩ ⟨(pl, pl'), hpl⟩
      ⟨(p1bs, p1bs'), hp1bs⟩ f ⟨(mx, mx'), hmx⟩ (spdecMR dec)).val.2.1.property,
   (sequence_payload (MonoRel L mem) variant prop acc ⟨(cp, cp'), hcp⟩
      ⟨(ck, ck'), hck⟩ ⟨(pb, pb'), hpb⟩ ⟨(pl, pl'), hpl⟩
      ⟨(p1bs, p1bs'), hp1bs⟩ f ⟨(mx, mx'), hmx⟩ (spdecMR dec)).val.2.2.property⟩

/-! ## Executable smoke test: one proposer, one acceptor, f = 0 —
a payload flows through sequencing, acceptance, quorum, and lands at
the replica. -/

#guard (sequence_payload (Values PaxLoc (paxMem 1 1)) .guarded .prop .acc
    (fun _ => [42])                                   -- client payload
    (fun _ => [none, none])                           -- a_checkpoint
    (fun _ => [Ballot.mk 0 0, Ballot.mk 0 0])         -- p_ballot
    (fun _ => [true, true])                           -- p_is_leader
    (fun _ => [{}, {}])                               -- p_relevant_p1bs
    0
    (fun _ => [some (Ballot.mk 0 0), some (Ballot.mk 0 0)])  -- a_max
    ⟨fun _ => [1, 0], (),                             -- payload batch
     (), fun _ => [{⟨0, Ballot.mk 0 0, 0, some 42⟩}, {}],  -- p2a batches
     (), fun _ => [{((0, Ballot.mk 0 0), .ok ())}, {}], (),  -- cq acks
     fun _ => [{((0, Ballot.mk 0 0), ())}, {}], ()⟩    -- jr key batches
    ).val.1 0
  = ({(0, some 42)} : Multiset (Nat × Option Nat))

end HydroV2
