import Hydro.Paxos.PaxosWithClient
import Hydro.Paxos.KvReplica
import Hydro.Std.BenchClient
import Hydro.Std.Quorum

/-!
# `hydro_test/src/cluster/paxos_bench.rs` — the Paxos benchmark

One Rust function = one Lean definition: `inc_i32_workload_generator`
(:138–148), `paxos_bench` (:16–135) — plus `acceptor_checkpoint_from_replicas`
for the `let a_checkpoint = { … sliced! { … } }` block (:69–96), written
as the function Rust would have (it is the one place `paxos_bench` reasons
about its own content: the checkpoint optional's latests must ascend for
`paxos_core` to read it as `Monotonic`; stating that as the block's
`ensures` keeps the proof where the program is).

**Closures, flattened (ruling F).** Rust's `bench_client` takes the
workload generator and the protocol as closures and threads two
`KeyedStream` cycles through them; `paxos_bench`'s body here IS those
closures' bodies, the two cycles one `fix … complete` over the keyed
streams' pair spellings (`kentries`/`into_keyed` are the casts — the
engine's knot carriers are streams). `with_client`'s own closure is
`payloads_to_leader` (PaxosWithClient). `print_results` is the caller's:
the function returns what Rust hands it.

**The acceptor checkpoint** is `acceptors.forward_ref::<Optional<_, _, _>>()`
closed by `complete(a_checkpoint)`: a tick knot on the checkpoint trace,
read by `paxos_core` through `latest_monotone` (`Optional::latest()` as a
`Monotonic` singleton — the knot invariant is the ascent of the update trace,
`none ⊑ some x ⊑ some y`, its `ghost pre` needs, discharged by the block's
`ensures` at every stage).
-/

namespace Hydro

variable {L : Type} {mem : L → Nat}

/-! ## The ascent of a quorum minimum along cumulative views -/

/-- A trace whose `some`s are pairwise ascending (every earlier `some` is
below every later one, which is then a `some` too) ascends in the `none`-bottomed
order. -/
theorem Ascending.of_pairwise_some {tr : Trace (Option Nat)}
    (h : ∀ (t t' : Nat) (ht' : t' < tr.length) (hlt : t < t'),
      ∀ a, tr[t]'(Nat.lt_trans hlt ht') = some a →
        ∃ b, tr[t'] = some b ∧ a ≤ b) :
    Ascending ValueOrder.optNat tr := by
  intro t t' hle ht'
  rcases Nat.lt_or_eq_of_le hle with hlt | rfl
  · rcases hx : tr[t]'(Nat.lt_of_le_of_lt hle ht') with _ | a
    · trivial
    · obtain ⟨b, hb, hab⟩ := h t t' ht' hlt a hx
      rw [hb]; exact hab
  · exact ValueOrder.optNat.le_refl _

/-- `.min()`'s running fold never returns `None` once fed. -/
theorem foldl_minStep_ne_none {α : Type} [LinearOrder α] (m : Multiset α) :
    ∀ init, Multiset.foldl minStep init m = none → init = none ∧ m = 0 := by
  induction m using Multiset.induction_on with
  | empty => intro init h; exact ⟨by simpa [Multiset.foldl_zero] using h, rfl⟩
  | cons x s ih =>
    intro init h
    rw [Multiset.foldl_cons] at h
    obtain ⟨h1, -⟩ := ih _ h
    exact absurd h1 (by simp [minStep])

/-- What `.min()` returns is a member below every member (or the seed,
below every member). -/
theorem foldl_minStep_spec {α : Type} [LinearOrder α] (m : Multiset α) :
    ∀ init a, Multiset.foldl minStep init m = some a →
      (a ∈ m ∨ init = some a) ∧ (∀ x ∈ m, a ≤ x) ∧ (∀ y, init = some y → a ≤ y) := by
  induction m using Multiset.induction_on with
  | empty =>
    intro init a h
    rw [Multiset.foldl_zero] at h
    refine ⟨Or.inr h, fun x hx => absurd hx (Multiset.notMem_zero x), fun y hy => ?_⟩
    rw [h] at hy; cases hy; exact le_refl _
  | cons x s ih =>
    intro init a h
    rw [Multiset.foldl_cons] at h
    obtain ⟨h1, h2, h3⟩ := ih _ a h
    cases init with
    | none =>
      have hx : a ≤ x := h3 x rfl
      refine ⟨?_, fun z hz => ?_, fun y hy => by cases hy⟩
      · rcases h1 with h1 | h1
        · exact Or.inl (Multiset.mem_cons_of_mem h1)
        · simp only [minStep, Option.some.injEq] at h1
          exact Or.inl (h1 ▸ Multiset.mem_cons_self x s)
      · rcases Multiset.mem_cons.mp hz with rfl | hz
        · exact hx
        · exact h2 z hz
    | some y =>
      have hmin : a ≤ Min.min y x := h3 _ rfl
      refine ⟨?_, fun z hz => ?_, fun y' hy' => ?_⟩
      · rcases h1 with h1 | h1
        · exact Or.inl (Multiset.mem_cons_of_mem h1)
        · simp only [minStep, Option.some.injEq] at h1
          rcases le_total y x with hyx | hxy
          · exact Or.inr (by rw [← h1, min_eq_left hyx])
          · exact Or.inl (by rw [← h1, min_eq_right hxy]; exact Multiset.mem_cons_self x s)
      · rcases Multiset.mem_cons.mp hz with rfl | hz
        · exact le_trans hmin (min_le_right _ _)
        · exact h2 z hz
      · cases hy'
        exact le_trans hmin (min_le_left _ _)

/-- The Rust reduce closure `|curr_seq, seq| if seq > *curr_seq { *curr_seq = seq }`. -/
def keepLarger (curr_seq seq : Nat) : Nat := if seq > curr_seq then seq else curr_seq

theorem le_foldl_keepLarger (s : Nat) (l : List Nat) : s ≤ l.foldl keepLarger s := by
  induction l generalizing s with
  | nil => exact le_refl _
  | cons x xs ih =>
    refine le_trans ?_ (ih _)
    unfold keepLarger; split <;> omega

/-- The running maximum along a prefix is below that along the list. -/
theorem foldl_keepLarger_prefix_le {l l' : List Nat} (h : l <+: l') :
    l.foldl keepLarger 0 ≤ l'.foldl keepLarger 0 := by
  obtain ⟨r, rfl⟩ := h
  rw [List.foldl_append]
  exact le_foldl_keepLarger _ _

/-- The per-view read of `acceptor_checkpoint_from_replicas`'s tick: the
minimum of the per-sender maxima, when every one of the `f + 1` senders
has reported. -/
def checkpointQuorumMin {nR : Nat} (f : Nat) (view : KeyedMap (Fin nR) Nat) : Option Nat :=
  poolMin (ord := .noOrder) (mapPool (ord := .noOrder) (fun (p : Fin nR × Nat) => p.2)
    (poolFilterIf (ord := .noOrder) view.entries (decide (view.keyCount = f + 1))))

/-- A value `.min()` returns over a map's entries is one of its values. -/
theorem mem_entries_values {K : Type} [DecidableEq K] (m : KeyedMap K Nat) {a : Nat}
    (ha : a ∈ Multiset.map Prod.snd m.entries) : ∃ k ∈ m.keys, m.get k = some a := by
  obtain ⟨⟨k, a'⟩, hka, rfl⟩ := Multiset.mem_map.mp ha
  simp only [KeyedMap.entries, Multiset.mem_filterMap] at hka
  obtain ⟨k', hk', hget⟩ := hka
  cases hv : m.get k' with
  | none => rw [hv] at hget; cases hget
  | some v =>
    rw [hv] at hget
    simp only [Option.map_some, Option.some.injEq, Prod.mk.injEq] at hget
    obtain ⟨rfl, rfl⟩ := hget
    exact ⟨k', hk', hv⟩

theorem mem_entries_values_of_get {K : Type} [DecidableEq K] (m : KeyedMap K Nat) {k : K} {a : Nat}
    (hget : m.get k = some a) : a ∈ Multiset.map Prod.snd m.entries := by
  refine Multiset.mem_map.mpr ⟨(k, a), ?_, rfl⟩
  simp only [KeyedMap.entries, Multiset.mem_filterMap]
  exact ⟨k, (m.dom k).mp (by rw [hget]; rfl), by rw [hget]; rfl⟩

/-- **The checkpoint minimum ascends along cumulative views** (with exactly
`f + 1` replicas): once every replica has reported, later views keep every
key and only raise each key's maximum, so the minimum over them rises. -/
theorem checkpointQuorumMin_ascends {nR f : Nat} (hR : nR = f + 1)
    (p : KeyedPool (Fin nR) Nat .totalOrder .exactlyOnce)
    {M M' : Multiset (Fin nR × Nat)} (hMM : M ≤ M')
    (Pre : Nat → Nat → Prop) [DecidableRel Pre] (ok : FoldOkP .totalOrder .exactlyOnce Pre keepLarger)
    {a : Nat}
    (ha : checkpointQuorumMin f (KeyedMap.ofFold keepLarger 0 Pre ok (p.readAt M)) = some a) :
    ∃ b, checkpointQuorumMin f (KeyedMap.ofFold keepLarger 0 Pre ok (p.readAt M')) = some b ∧ a ≤ b := by
  set v := KeyedMap.ofFold keepLarger 0 Pre ok (p.readAt M) with hv
  set v' := KeyedMap.ofFold keepLarger 0 Pre ok (p.readAt M') with hv'
  -- the gate was open at `M`: every replica had reported
  have hgate : decide (v.keyCount = f + 1) = true := by
    by_contra hng
    rw [Bool.not_eq_true] at hng
    simp only [checkpointQuorumMin, hng, poolFilterIf, Bool.false_eq_true, if_false] at ha
    change Multiset.foldl minStep none (Multiset.map Prod.snd (PoolBot (α := Fin nR × Nat) .noOrder .exactlyOnce)) = some a at ha
    simp [PoolBot, Multiset.foldl_zero] at ha
  have hcard : v.keys.card = nR := by
    have h := of_decide_eq_true hgate
    change v.keys.card = f + 1 at h
    omega
  have hkeys : v.keys = Finset.univ := by
    rw [← Finset.card_eq_iff_eq_univ, Fintype.card_fin]; exact hcard
  -- the keys only grow: the gate stays open at `M'`
  have hkeys' : v'.keys = Finset.univ := by
    apply Finset.eq_univ_of_forall
    intro k
    have hk : k ∈ v.keys := by rw [hkeys]; exact Finset.mem_univ k
    change k ∈ (p.readAt M).keys at hk
    change k ∈ (p.readAt M').keys
    simp only [KeyedPool.readAt, Multiset.mem_toFinset] at hk ⊢
    exact Multiset.subset_of_le (Multiset.map_le_map hMM) hk
  have hgate' : decide (v'.keyCount = f + 1) = true := by
    rw [decide_eq_true_eq, KeyedMap.keyCount, hkeys', Finset.card_univ, Fintype.card_fin]; exact hR
  -- per key the maximum rises
  have hrise : ∀ k x, v.get k = some x → ∃ y, v'.get k = some y ∧ x ≤ y := by
    intro k x hx
    have hk' : k ∈ v'.keys := by rw [hkeys']; exact Finset.mem_univ k
    change k ∈ (p.readAt M').keys at hk'
    have hk : k ∈ (p.readAt M).keys := by
      have : k ∈ v.keys := by rw [hkeys]; exact Finset.mem_univ k
      exact this
    simp only [hv, hv', KeyedMap.ofFold, hk, hk', if_true, Option.some.injEq] at hx ⊢
    refine ⟨_, rfl, ?_⟩
    rw [← hx]
    exact foldl_keepLarger_prefix_le (KeyedPool.readAt_grp_prefix_of_le p hMM k)
  -- the minimum at `M` is some key's value, below every key's value
  simp only [checkpointQuorumMin, hgate, hgate', poolFilterIf, if_true] at ha ⊢
  change Multiset.foldl minStep none (Multiset.map Prod.snd v.entries) = some a at ha
  change ∃ b, Multiset.foldl minStep none (Multiset.map Prod.snd v'.entries) = some b ∧ a ≤ b
  obtain ⟨hmem, hle, -⟩ := foldl_minStep_spec _ none a ha
  have hmem' : a ∈ Multiset.map Prod.snd v.entries := by
    rcases hmem with h | h
    · exact h
    · cases h
  -- the minimum at `M'` exists (the entries are nonempty) and is some key's value there
  obtain ⟨b, hb⟩ : ∃ b, Multiset.foldl minStep none (Multiset.map Prod.snd v'.entries) = some b := by
    cases hfold : Multiset.foldl minStep none (Multiset.map Prod.snd v'.entries) with
    | some b => exact ⟨b, rfl⟩
    | none =>
      exfalso
      obtain ⟨-, hz⟩ := foldl_minStep_ne_none _ none hfold
      obtain ⟨k, -, hk⟩ := mem_entries_values v hmem'
      obtain ⟨y, hy, -⟩ := hrise k a hk
      have := mem_entries_values_of_get v' hy
      rw [hz] at this
      exact Multiset.notMem_zero y this
  refine ⟨b, hb, ?_⟩
  obtain ⟨hmemb, -, -⟩ := foldl_minStep_spec _ none b hb
  have hmemb' : b ∈ Multiset.map Prod.snd v'.entries := by
    rcases hmemb with h | h
    · exact h
    · cases h
  obtain ⟨k₀, -, hk₀⟩ := mem_entries_values v' hmemb'
  -- `k₀`'s value at `M` is above `a` and below its value at `M'` (= `b`)
  have hk₀M : ∃ x, v.get k₀ = some x := by
    have : k₀ ∈ v.keys := by rw [hkeys]; exact Finset.mem_univ k₀
    rcases hx : v.get k₀ with _ | x
    · exact absurd ((v.dom k₀).mpr this) (by rw [hx]; simp)
    · exact ⟨x, rfl⟩
  obtain ⟨x, hx⟩ := hk₀M
  obtain ⟨y, hy, hxy⟩ := hrise k₀ x hx
  rw [hk₀] at hy
  cases hy
  exact le_trans (hle x (mem_entries_values_of_get v hx)) hxy

/-- The block's decisions: its one `nondet!`. -/
structure ACQDec (H : HydroSem L mem) (nA nR : Nat) where
  /-- `use::snapshot(a_checkpoint_largest_seqs, nondet!(/** … the final min checkpoint is deterministic */))` (:81–84). -/
  snap : H.SnapDec nA (Fin nR × Nat) .noOrder

/-- What the block **ensures**: with exactly `f + 1` replicas, the
checkpoint optional's latests ascend (what `paxos_core` reads it as —
`Monotonic`). The gate is `== f + 1` (SCHED_AUDIT: with more replicas it
closes forever once all report, and a late replica's lower checkpoint
could otherwise lower the minimum). -/
structure ACQEnsures (nR f : Nat) {nA : Nat} (out : MemberMap nA (Trace (Option Nat))) : Prop where
  ascending : nR = f + 1 → ∀ i, Ascending ValueOrder.optNat (out i)

/-- **paxos_bench.rs:69–96**, the `a_checkpoint` block, as the function
Rust would write (`fn acceptor_checkpoint_from_replicas(replica_checkpoint,
acceptors, f) -> Optional<usize, Cluster<Acceptor>, Unbounded>`): each
acceptor keeps every replica's largest checkpoint and, once all `f + 1`
have reported, emits the smallest — the slot every replica has passed. -/
hydro def acceptor_checkpoint_from_replicas (H : HydroSem L mem) (replicas acceptors : L) (f : Nat)
    (replica_checkpoint : H.Stream replicas Nat .totalOrder .exactlyOnce)
    (dec : ACQDec H (mem acceptors) (mem replicas))
    (sched : H.TransportDec (mem acceptors) (mem replicas)) :
    H.Atomic acceptors (Option Nat)
  ensures out => ACQEnsures (mem replicas) f out :=
  -- let a_checkpoint_largest_seqs = replica_checkpoint
  --   .broadcast(&acceptors, TCP.fail_stop().bincode(), nondet!(/** TODO */))
  --   .reduce(q!(|curr_seq, seq| { if seq > *curr_seq { *curr_seq = seq; } },
  --     commutative = manual_proof!(/** max is commutative */)));
  --   — per sender the broadcast keeps the replica's order (`TotalOrder`): the reduce's
  --   obligation is trivial here; `reduce` from `0` is the fold (checkpoints are `usize`)
  let a_checkpoint_largest_seqs := H.kfold keepLarger 0
    (fun _ _ => True) trivial (H.broadcast_closed sched replica_checkpoint)
  -- sliced! {
  --   let snapshot = use::snapshot(a_checkpoint_largest_seqs, nondet!(…));
  tick (input snapshot := H.ksnapshot a_checkpoint_largest_seqs dec.snap) :=
    -- let a_checkpoints_quorum_reached = snapshot.clone().key_count()
    --   .map(q!(move |num_received| num_received == f + 1));
    let a_checkpoints_quorum_reached := H.bsMap (H.bksKeyCount snapshot)
      (fun num_received => decide (num_received = f + 1))
    -- // Find the smallest checkpoint seq that everyone agrees to
    -- snapshot.entries().filter_if(a_checkpoints_quorum_reached).map(q!(|(_sender, seq)| seq)).min()
    emit (a_checkpoint := H.bmin (H.bmap
      (H.bfilterIf (H.bksEntries snapshot) a_checkpoints_quorum_reached)
      (fun (_sender, seq) => seq)));
  -- }
  a_checkpoint
  prove ascending := by
    intro hreq i
    -- the tick's input trace: the cumulative views of the acceptor's
    -- per-replica pool, folded per key
    have hsnap : ∀ (n : Nat) v, (MemberMap.get snapshot i)[n]? = some v ↔
        ∃ M, (KeyedPool.kviews (MemberMap.get ((Values L mem).broadcast_closed sched replica_checkpoint) i)
            0 (dec.snap i))[n]? = some M
          ∧ v = KeyedMap.ofFold keepLarger 0 (fun _ _ => True) trivial
              (KeyedPool.readAt (MemberMap.get ((Values L mem).broadcast_closed sched replica_checkpoint) i) M) := by
      intro n v
      show ((KeyedPool.kviews _ 0 _).map _)[n]? = some v ↔ _
      rw [List.getElem?_map]
      constructor
      · intro h
        obtain ⟨M, hM, rfl⟩ := Option.map_eq_some_iff.mp h
        exact ⟨M, hM, rfl⟩
      · rintro ⟨M, hM, rfl⟩
        rw [hM]; rfl
    have hlen : (MemberMap.get a_checkpoint i).length = (MemberMap.get snapshot i).length := by
      rw [ha_checkpoint_run]; exact scanAcrossTicksTrace_length _ _ _
    apply Ascending.of_pairwise_some
    intro t t' ht' hlt a ha
    -- the reads at `t` and `t'`
    have ha? : (MemberMap.get a_checkpoint i)[t]? = some (some a) := by
      rw [List.getElem?_eq_getElem (Nat.lt_trans hlt ht')]; exact congrArg some ha
    obtain ⟨vt, hvt, he⟩ := (ha_checkpoint_at i t (some a)).mp ha?
    obtain ⟨vt', hvt'⟩ : ∃ v, (MemberMap.get snapshot i)[t']? = some v :=
      ⟨_, List.getElem?_eq_getElem (by rw [← hlen]; exact ht')⟩
    have ha'? := (ha_checkpoint_at i t' _).mpr ⟨vt', hvt', rfl⟩
    obtain ⟨M, hM, rfl⟩ := (hsnap t vt).mp hvt
    obtain ⟨M', hM', rfl⟩ := (hsnap t' vt').mp hvt'
    have hMM : M ≤ M' := KeyedPool.kviews_le _ 0 _ (Nat.le_of_lt hlt) hM hM'
    -- the per-view read is the quorum minimum
    have hread : ∀ st v, (a_checkpoint_step i st v).2 = checkpointQuorumMin f v := fun _ _ => rfl
    simp only [hread] at he ha'?
    obtain ⟨b, hb, hab⟩ := checkpointQuorumMin_ascends hreq _ hMM _ _ he.symm
    refine ⟨b, ?_, hab⟩
    rw [hb] at ha'?
    exact (List.getElem?_eq_some_iff.mp ha'?).2

#nondet_census acceptor_checkpoint_from_replicas (nondets := 1) (scheds := 1) (fuels := 0)

/-- **paxos_bench.rs:138–148 `inc_i32_workload_generator`**: an incrementing
counter per virtual client, from 0. -/
hydro def inc_i32_workload_generator (H : HydroSem L mem) (clients : L)
    (ids_and_prev_payloads : H.KeyedStream clients Nat (Option Int) .noOrder .exactlyOnce) :
    H.KeyedStream clients Nat Int .noOrder .exactlyOnce :=
  -- ids_and_prev_payloads.map(q!(move |payload| { if let Some(counter) = payload { counter + 1 } else { 0 } }))
  H.kmap ids_and_prev_payloads (fun _me payload =>
    match payload with
    | some counter => counter + 1
    | none => 0)

#nondet_census inc_i32_workload_generator (nondets := 0) (scheds := 0) (fuels := 0)

/-- The benchmark's payload: `(virtual_id, (CLUSTER_SELF_ID, payload))` (:38–42).
The value is a Rust tuple, whose `Ord` is lexicographic — `Lex` (the KV
store's sort needs the value's order, KvReplica). -/
abbrev BenchValue (nC : Nat) : Type := Lex (Fin nC × Int)
abbrev BenchPayload (nC : Nat) : Type := Nat × BenchValue nC

instance {α : Type} [Wire α] : Wire (Lex α) := Wire.ofEquiv ofLex toLex (fun _ => rfl)

/-- **paxos_bench.rs:38–43**, the protocol closure's payload expression, as
the function Rust would write (`fn tag_with_sender(input: KeyedStream<u32,
i32, Cluster<Client>, Unbounded, NoOrder>) -> Stream<(u32, (MemberId<Client>,
i32)), Cluster<Client>, Unbounded, TotalOrder>`): append the client id so
replicas know whom to acknowledge, and pick an order (the engine's knot
bodies consume their content decisions through callees — ENGINE_NOTES). -/
hydro def tag_with_sender (H : HydroSem L mem) (clients : L)
    (input : H.KeyedStream clients Nat Int .noOrder .exactlyOnce)
    (dec : H.OrderSelDec (mem clients) (BenchPayload (mem clients))) :
    H.Stream clients (BenchPayload (mem clients)) .totalOrder .exactlyOnce :=
  -- input.entries().map(q!(move |(virtual_id, payload)| {
  --   // Append Client ID so replicas know who to contact later
  --   (virtual_id, (CLUSTER_SELF_ID.clone(), payload)) }))
  -- .assume_ordering(nondet!(/** benchmarking, order actually doesn't matter */))
  H.assume_ordering
    (H.map (H.kentries input) (fun me (virtual_id, payload) => (virtual_id, toLex (me, payload))))
    dec

#nondet_census tag_with_sender (nondets := 1) (scheds := 0) (fuels := 0)

/-- The protocol closure's decisions: its callees' and its own `nondet!` sites and knot. -/
structure PPDec (H : HydroSem L mem) (nP nA nC nR : Nat) where
  /-- `.assume_ordering(nondet!(/** benchmarking, order actually doesn't matter */))` (:43). -/
  order : H.OrderSelDec nC (BenchPayload nC)
  /-- `paxos.with_client(…)`'s (`paxos_core`'s and the client closure's). -/
  pwc : PWCDec H nP nA nC (BenchPayload nC) (Option Nat) .totalOrder
  /-- `kv_replica`'s. -/
  kv : KvRDec H nR Nat (BenchValue nC)
  /-- The `a_checkpoint` block's. -/
  ckpt : ACQDec H nA nR
  /-- `collect_quorum`'s `use::batch(responses, nondet!(…))`. -/
  cq : H.BatchDec nC (((Nat × Int) × Except Unit Unit)) .noOrder .exactlyOnce
  /-- `acceptors.forward_ref::<Optional<_, _, _>>()` + `acceptor_checkpoint_complete.complete(a_checkpoint)` (:34–35, :98). -/
  fuelCheckpoint : H.FixDec

/-- The protocol closure's adversary-side (sched-det) bundle. -/
structure PPSched (H : HydroSem L mem) (nP nA nC nR : Nat) where
  pwc : PWCSched H nP nA nC (BenchPayload nC)
  /-- `sequenced_payloads.broadcast(replicas, TCP.fail_stop().bincode(), nondet!(/** TODO */))` (:57). -/
  toReplicas : H.TransportDec nR nP
  /-- `replica_checkpoint.broadcast(&acceptors, TCP.fail_stop().bincode(), nondet!(/** TODO */))` (:71). -/
  ckpt : H.TransportDec nA nR
  /-- `.demux(clients, TCP.fail_stop().bincode())` (:105). -/
  toClients : H.TransportDec nC nR

/-- The trivial bundle at the denotation. -/
def PPSched.triv {nP nA nC nR : Nat} : PPSched (Values L mem) nP nA nC nR := ⟨.triv, (), (), ()⟩

/-- **paxos_bench.rs:32–120**, the protocol closure `|input| { … }` handed to
`bench_client`, as the function Rust would write (`fn paxos_protocol(input:
KeyedStream<u32, i32, Cluster<Client>, Unbounded, NoOrder>) -> KeyedStream<u32,
i32, Cluster<Client>, Unbounded, NoOrder>`): the clients' payloads go through
Paxos (`with_client`), the sequenced log is applied by `replicas`
(`kv_replica`), whose checkpoints feed the acceptors' truncation and whose
acknowledgements, once `f + 1` replicas agree, complete each request.
`requires`: the guarded `paxos_core` variant (its own clause) and exactly
`f + 1` replicas (the `== f + 1` checkpoint gate's premise — the acceptors'
`Monotonic` read of the checkpoint, the knot invariant). -/
hydro def paxos_protocol (H : HydroSem L mem) (variant : PaxosVariant)
    (prop acc clients replicas : L)
    (checkpoint_frequency f num_replicas : Nat) (paxos_config : PaxosConfig)
    (input : H.KeyedStream clients Nat Int .noOrder .exactlyOnce)
    (dec : PPDec H (mem prop) (mem acc) (mem clients) (mem replicas))
    (sched : PPSched H (mem prop) (mem acc) (mem clients) (mem replicas)) :
    H.KeyedStream clients Nat Int .noOrder .exactlyOnce
  requires (variant = .guarded ∧ mem replicas = f + 1) :=
  -- let acceptors = paxos.log_stores().clone();
  -- let (acceptor_checkpoint_complete, acceptor_checkpoint) = acceptors.forward_ref::<Optional<_, _, _>>();
  --   — the checkpoint knot: `paxos_core` reads it as `Monotonic` (`latest_monotone`), so
  --   its latests ascend — the loop invariant, the `a_checkpoint` block's `ensures` at
  --   every stage (under this module's clause)
  fix (acceptor_checkpoint : H.Atomic acc (Option Nat)) via dec.fuelCheckpoint
    invariant acceptor_checkpoint (kv a_checkpoint) =>
      (mem replicas = f + 1 → ∀ i, Ascending ValueOrder.optNat (acceptor_checkpoint i)),
      base := by
        -- the seed stage: the empty trace
        intros
        exact fun _ ht' => absurd ht' (by simp),
      step := by
        -- the block's `ensures`, at this stage's `kv`
        intros
        rename_i hR i
        exact (acceptor_checkpoint_from_replicas.ensures _ _ _ _ _ _).ascending hR i :=
    ghost pre hck : ∀ i, Ascending ValueOrder.optNat (acceptor_checkpoint i) := hacceptor_checkpoint_inv hreq.2
    let a_checkpoint_mono := H.latest_monotone acceptor_checkpoint
    -- `with_client`'s clause (the guarded variant: this module's own)
    ghost pre hvar : variant = .guarded := hreq.1
    -- let sequenced_payloads = paxos.with_client(clients,
    --   input.entries().map(q!(move |(virtual_id, payload)| { (virtual_id, (CLUSTER_SELF_ID.clone(), payload)) }))
    --     .assume_ordering(nondet!(/** benchmarking, order actually doesn't matter */)),
    --   acceptor_checkpoint, nondet!(/** benchmarking, assuming no re-election */), nondet!(/** clients 'own' certain keys … */));
    let sequenced_payloads := paxos_with_client H variant prop acc clients paxos_config
      (tag_with_sender H clients input dec.order) a_checkpoint_mono dec.pwc sched.pwc
    -- let sequenced_to_replicas = sequenced_payloads
    --   .broadcast(replicas, TCP.fail_stop().bincode(), nondet!(/** TODO */)).values()
    --   .map(q!(|(index, payload)| (index, payload.map(|(key, value)| KvPayload { key, value }))));
    let sequenced_to_replicas := H.map (H.values (H.broadcast_closed sched.toReplicas sequenced_payloads))
      (fun _me (index, payload) =>
        (index, payload.map (fun (key, value) => (⟨key, value⟩ : KvPayload Nat (BenchValue (mem clients))))))
    -- // Replicas
    -- let (replica_checkpoint, processed_payloads) = kv_replica(replicas, sequenced_to_replicas, checkpoint_frequency);
    let kv := kv_replica H replicas sequenced_to_replicas checkpoint_frequency dec.kv
    -- // Get the latest checkpoint sequence per replica
    -- let a_checkpoint = { … };
    let a_checkpoint := acceptor_checkpoint_from_replicas H replicas acc f kv.1 dec.ckpt sched.ckpt
    -- let c_received_payloads = processed_payloads
    --   .map(q!(|payload| (payload.value.0, ((payload.key, payload.value.1), Ok(())))))
    --   .demux(clients, TCP.fail_stop().bincode()).values();
    let c_received_payloads := H.values (H.demux sched.toClients
      (H.map kv.2 (fun _me payload =>
        ((ofLex payload.value).1.val, ((payload.key, (ofLex payload.value).2), (Except.ok () : Except Unit Unit)))))
      (fun i => i.val))
    -- // we only mark a transaction as committed when all replicas have applied it
    -- collect_quorum::<_, _, _, ()>(c_received_payloads, f + 1, num_replicas).0.into_keyed()
    let cq := collect_quorum H clients c_received_payloads (f + 1) num_replicas dec.cq
  -- acceptor_checkpoint_complete.complete(a_checkpoint);
  complete (a_checkpoint)
  H.into_keyed cq.1

#nondet_census paxos_protocol (nondets := 23) (scheds := 12) (fuels := 9)

/-- `paxos_bench`'s decisions: its callees' and its two `bench_client` cycles. -/
structure PBDec (H : HydroSem L mem) (nP nA nC nR nAg : Nat) where
  /-- `bench_client`'s. -/
  bench : BCDec H nC Int Int
  /-- The protocol closure's. -/
  protocol : PPDec H nP nA nC nR
  /-- `compute_throughput_latency`'s. -/
  ctl : CTLDec H nC
  /-- `aggregate_bench_results`'s. -/
  abr : ABRDec H nAg
  /-- `bench_client`'s `protocol_inputs` cycle (the workload generator's output feeds the protocol). -/
  fuelInputs : H.FixDec
  /-- `bench_client`'s `protocol_outputs` cycle (the protocol's acknowledgements feed the client). -/
  fuelOutputs : H.FixDec

/-- `paxos_bench`'s adversary-side (sched-det) bundle. -/
structure PBSched (H : HydroSem L mem) (nP nA nC nR nAg : Nat) where
  protocol : PPSched H nP nA nC nR
  /-- `compute_throughput_latency`'s measurement-window timer. -/
  ctl : CTLSched H nC
  abr : ABRSched H nC nAg

/-- The trivial bundle at the denotation. -/
def PBSched.triv {nP nA nC nR nAg : Nat} : PBSched (Values L mem) nP nA nC nR nAg := ⟨.triv, .triv, .triv⟩

/-- **paxos_bench.rs:16–135 `paxos_bench`**: virtual clients on `clients`
drive incrementing payloads through the protocol (`paxos_protocol`);
throughput/latency are measured at the clients and aggregated at
`aggregator`. Returns what Rust hands `print_results`. `requires`: the
protocol's clause (the guarded `paxos_core` variant, exactly `f + 1`
replicas). -/
hydro def paxos_bench (H : HydroSem L mem) (variant : PaxosVariant)
    (prop acc clients aggregator replicas : L)
    (checkpoint_frequency f num_replicas num_clients_per_node : Nat) (paxos_config : PaxosConfig)
    (client_interval_millis aggregate_interval_millis : Nat)
    (dec : PBDec H (mem prop) (mem acc) (mem clients) (mem replicas) (mem aggregator))
    (sched : PBSched H (mem prop) (mem acc) (mem clients) (mem replicas) (mem aggregator)) :
    (H.Stream aggregator Nat .totalOrder .exactlyOnce
      × H.Stream aggregator Histogram .totalOrder .exactlyOnce)
  requires (variant = .guarded ∧ mem replicas = f + 1) :=
  -- let latencies = bench_client(clients, num_clients_per_node, inc_i32_workload_generator, |input| { … })
  --   — the two closures' cycles through `bench_client` (F): the generator's output is the
  --   protocol's input, the protocol's result the client's `protocol_outputs`; keyed streams
  --   knot as their pair streams (`into_keyed`/`kentries` are the casts)
  fix (protocol_inputs_pairs : H.Stream clients (Nat × Int) .noOrder .exactlyOnce)
      (protocol_outputs_pairs : H.Stream clients (Nat × Int) .noOrder .exactlyOnce)
      via (dec.fuelInputs, dec.fuelOutputs) :=
    let bc := bench_client H clients num_clients_per_node
      (H.into_keyed protocol_inputs_pairs) (H.into_keyed protocol_outputs_pairs) dec.bench
    -- the workload generator, at `bench_client`'s `ids_and_prev_payloads`
    let input := inc_i32_workload_generator H clients bc.1
    -- the protocol closure, at the generator's output (its clause is this module's own)
    ghost pre hprot : variant = .guarded ∧ mem replicas = f + 1 := hreq
    let protocol_outputs := paxos_protocol H variant prop acc clients replicas
      checkpoint_frequency f num_replicas paxos_config input dec.protocol sched.protocol
    -- the lines after `bench_client` returns, stated here inside the knot body (they read
    -- `bench_client`'s latencies, which the knot carries; the engine's knot continuation
    -- carries no decisions of its own — ENGINE_NOTES)
    -- .entries().map(q!(|(_virtual_client_id, (_output, latency))| latency));
    let latencies := H.map (H.kentries bc.2) (fun _me (_virtual_client_id, (_output, latency)) => latency)
    -- // Create throughput/latency graphs
    -- let bench_results = compute_throughput_latency(clients, latencies, client_interval_millis, nondet!(/** bench */));
    let bench_results := compute_throughput_latency H clients latencies client_interval_millis dec.ctl sched.ctl
    -- let aggregate_results = aggregate_bench_results(bench_results, client_aggregator, aggregate_interval_millis);
    let aggregate_results := aggregate_bench_results H clients aggregator bench_results.1 bench_results.2
      aggregate_interval_millis dec.abr sched.abr
  -- the closures' results close `bench_client`'s cycles: the generator's payloads and the
  -- protocol's acknowledgements (as pairs — `.into_keyed()` is the knot's cast)
  complete (H.kentries input, H.kentries protocol_outputs)
  -- print_results(aggregate_results);   — the caller's: returned
  aggregate_results

#nondet_census paxos_bench (nondets := 34) (scheds := 16) (fuels := 11)

end Hydro
