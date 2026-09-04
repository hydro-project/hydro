import Hydro.Values
import Mathlib.Data.Multiset.Bind
import Hydro.HydroTick

/-!
# `hydro_std/src/request_response.rs` — `join_responses`

One Rust function = one Lean definition: join an incoming
request-response stream with metadata generated at request time.

**The atomic causality**: the metadata leg enters via
`use::atomic(metadata.all_ticks_atomic())` — it is synchronized with
the caller's tick, so metadata is available **immediately** (never a
stale batch that misses metadata sent before an async response came
back). In this signature that is a `TickStream` parameter in the
caller's tick domain — *not* a decision; the only `nondet!` is the
response `use::batch` (`BatchCuts`). The usage contract
(request_response.rs's doc): only one response element is produced
with a given key, same for the metadata stream.

Before the program: only its contract face (`JREnsures`). **The program
starts at `hydro def join_responses`**; its proof runs inline, reading the
`remaining_to_join` register through the construct's `_reg` and the body
through its generated step under the `den` simp set (FINDINGS D64).
-/

namespace Hydro

variable {L : Type} {mem : L → Nat}
variable {K M V : Type} [DecidableEq K] [DecidableEq M] [DecidableEq V]

/-! ## Prerequisites for the contract face -/

/-- What `join_responses` **ensures**, over the `Values` denotation:
`md` is the per-tick metadata (the caller's atomic tick wire), `dec`
the response batch cuts. -/
structure JREnsures (ℓ : L)
    (resp : Fin (mem ℓ) → Multiset (K × V))
    (md : Fin (mem ℓ) → Trace (Multiset (K × M)))
    (dec : BatchCuts (mem ℓ) (K × V))
    (out : Fin (mem ℓ) → Multiset (K × (M × V))) : Prop where
  /-- **Soundness** (unconditional): every joined output quotes its
  key's metadata and its key's consumed response. -/
  join_src : ∀ (i : Fin (mem ℓ)) {k : K} {m : M} {v : V},
    (k, (m, v)) ∈ out i →
    (k, m) ∈ (md i).sum ∧ (k, v) ∈ cqConsumed (resp i) (dec i)
  /-- **Completeness** (under the once-responder usage contract): a
  response consumed at tick `u` whose metadata was generated at or before
  `u` (the `atomic` availability — the metadata wire ticks at `u`) joins. -/
  join_complete : ∀ (i : Fin (mem ℓ)) {k : K} {m : M} {v : V},
    (((cqConsumed (resp i) (dec i)).map Prod.fst).count k ≤ 1) →
    ∀ {u : Nat} {rb : Multiset (K × V)} {mb : Multiset (K × M)},
      (batchCuts (resp i) 0 (dec i))[u]? = some rb → (md i)[u]? = some mb →
      (k, v) ∈ rb → (k, m) ∈ ((md i).take (u + 1)).sum →
      (k, (m, v)) ∈ out i

/-! ## The program -/

/-- **request_response.rs:15–43 `join_responses`** over location `ℓ`:
join each consumed response with its request-time metadata; unmatched
metadata persists (`remaining_to_join`).

Rust `nondet!` tally: 1 (the `use::batch`; the metadata leg is
`atomic` — the caller's tick domain, not a decision). -/
hydro def join_responses (H : HydroSem L mem) (ℓ : L)
    (responses : H.Stream ℓ (K × V) .noOrder .exactlyOnce)
    (metadata : H.TickStream ℓ (K × M) .noOrder .exactlyOnce)
    (dec : H.BatchDec (mem ℓ) (K × V)) :
    H.Stream ℓ (K × (M × V)) .noOrder .exactlyOnce
  ensures out => JREnsures ℓ responses metadata dec out :=
  -- sliced! {
  --   let mut remaining_to_join = use::state_null::<Stream<(K, M), _, _, NoOrder>>();
  --   let response_batch = use::batch(responses, nondet!(…));
  --   let metadata_batch = use::atomic(metadata.all_ticks_atomic(), nondet!(…));
  tick (state remaining_to_join : H.BoundedStream (K × M) .noOrder .exactlyOnce)
      (input response_batch := H.batch responses dec)
      (input metadata_batch := metadata) :=
    -- let remaining_and_new = remaining_to_join.chain(metadata_batch);
    let remaining_and_new := H.bchain remaining_to_join metadata_batch
    -- let joined_this_tick = remaining_and_new.clone().join(response_batch.clone())
    --   .map(q!(|(key, (md, resp))| (key, (md, resp))));
    let joined_this_tick := H.bmap (H.bjoin remaining_and_new response_batch)
      (fun (key, (md, resp)) => (key, (md, resp)))
    -- remaining_to_join = remaining_and_new.anti_join(response_batch.map(q!(|(key, _)| key)));
    rebind (remaining_to_join := H.bantiJoin remaining_and_new
      (H.bmap response_batch (fun (key, _) => key)))
    -- joined_this_tick
    yield (joined := joined_this_tick);
  -- }
  -- **the register, named** (`rem i n` = `remaining_to_join` before tick
  -- `n`): seeded empty, read at a tick, stepped, frozen once an input
  -- ends — the construct's `_reg`, the one place the block's fold is read
  ghost obtain ⟨rem, hrem_zero, hjoined_at, hrem_succ, hstall_resp, hstall_md⟩ := hjoined_reg
  -- **one tick, decoded**: a joined output quotes its key's metadata from
  -- the tick's window (the register plus this tick's metadata) and its
  -- key's response from this tick's batch
  ghost have hjoin_tick : ∀ (i : Fin (mem ℓ)) {n : Nat} {e : Multiset (K × (M × V))},
      (joined i)[n]? = some e →
      ∃ (rb : Multiset (K × V)) (mb : Multiset (K × M)),
        (response_batch i)[n]? = some rb ∧ (metadata_batch i)[n]? = some mb
        ∧ ∀ y ∈ e, (y.1, y.2.1) ∈ rem i n + mb ∧ (y.1, y.2.2) ∈ rb :=
    fun i {n e} he => by
    obtain ⟨rb, mb, hrb, hmb, rfl⟩ := (hjoined_at i n e).mp he
    refine ⟨rb, mb, hrb, hmb, fun y hy => ?_⟩
    simp only [joined_step, den, multisetJoin] at hy
    obtain ⟨z, hz, rfl⟩ := Multiset.mem_map.mp hy
    obtain ⟨km, hkm, hz2⟩ := Multiset.mem_bind.mp hz
    obtain ⟨kv, hkv, rfl⟩ := Multiset.mem_map.mp hz2
    obtain ⟨hkv, hkey⟩ := Multiset.mem_filter.mp hkv
    refine ⟨hkm, ?_⟩
    show ((km.1, kv.2) : K × V) ∈ rb
    rw [← hkey]
    exact hkv
  -- **the register only shrinks from the tick's window**: what it keeps
  -- is what the window held and this tick did not answer
  ghost have hrem_window : ∀ (i : Fin (mem ℓ)) {n : Nat} {rb : Multiset (K × V)}
      {mb : Multiset (K × M)},
      (response_batch i)[n]? = some rb → (metadata_batch i)[n]? = some mb →
      rem i (n + 1) = (rem i n + mb).filter (fun km => ∀ kv ∈ rb, kv.1 ≠ km.1) :=
    fun i {n rb mb} hrb hmb => by
    rw [hrem_succ i n rb mb hrb hmb]
    simp only [joined_step, den]
    refine Multiset.filter_congr fun km _ => ?_
    have hmem : ∀ (hd : Decidable (km.1 ∈ Multiset.map (fun x : K × V => x.1) rb)),
        ((!@decide _ hd) = true ↔ ∀ kv ∈ rb, kv.1 ≠ km.1) := by
      intro hd
      rw [Bool.not_eq_true', decide_eq_false_iff_not, Multiset.mem_map]
      simp only [not_exists, not_and, ne_eq]
    exact hmem _
  -- **the register holds metadata seen so far**: below the sum of the
  -- metadata ticks consumed before `n`
  ghost have hrem_sub : ∀ (i : Fin (mem ℓ)) (n : Nat),
      rem i n ≤ ((metadata i).take n).sum := fun i n => by
    induction n with
    | zero => rw [hrem_zero i]; exact Multiset.zero_le _
    | succ n ih =>
      have hmono : ((metadata i).take n).sum ≤ ((metadata i).take (n + 1)).sum :=
        sublist_sum_le (List.take_sublist_take_left (Nat.le_succ n))
      cases hrb : (response_batch i)[n]? with
      | none =>
        rw [hstall_resp i n hrb]
        exact le_trans ih hmono
      | some rb =>
        cases hmb : (metadata_batch i)[n]? with
        | none =>
          rw [hstall_md i n hmb]
          exact le_trans ih hmono
        | some mb =>
          rw [hrem_window i hrb hmb, List.take_add_one, List.sum_append]
          have hmb' : (metadata i)[n]? = some mb := hmb
          rw [hmb']
          simp only [Option.toList_some, List.sum_singleton]
          exact le_trans (Multiset.filter_le _ _) (add_le_add ih (le_refl _))
  -- all_ticks(joined_this_tick)
  H.allTicks joined
  prove
    join_src := fun i {k m v} hk => by
      -- the output is some tick's join: that tick's window holds the
      -- metadata, that tick's batch the response
      obtain ⟨e, he, hy⟩ := mem_list_sum.mp hk
      obtain ⟨t, ht⟩ := List.mem_iff_getElem?.mp he
      obtain ⟨rb, mb, hrb, hmb, hwin⟩ := hjoin_tick i ht
      obtain ⟨hmd, hresp⟩ := hwin _ hy
      refine ⟨?_, ?_⟩
      · -- the window sits below the metadata consumed through tick `t`,
        -- below the whole metadata wire
        have hmb' : (metadata i)[t]? = some mb := hmb
        have h1 : rem i t + mb ≤ ((metadata i).take (t + 1)).sum := by
          rw [List.take_add_one, List.sum_append, hmb']
          simp only [Option.toList_some, List.sum_singleton]
          exact add_le_add (hrem_sub i t) (le_refl _)
        exact Multiset.mem_of_le (le_trans h1 (sublist_sum_le (List.take_sublist _ _))) hmd
      · -- the batch is one of the consumed cuts
        exact mem_list_sum.mpr ⟨rb, Trace.mem_of_read hrb, hresp⟩,
    join_complete := fun i {k m v} honce {u rb mb} hrb hmb hresp hmd => by
      -- no earlier tick answered `k` (once-responder: two answering ticks
      -- would put two `k`-responses in the consumed pool)
      have hno : ∀ w, w < u → ∀ rb', (batchCuts (responses i) 0 (dec i))[w]? = some rb' →
          ∀ kv ∈ rb', kv.1 ≠ k := by
        intro w hw rb' hrb' kv hkv hkey
        have hle := Trace.two_reads_le_sum hw hrb' hrb
        have hcount := Multiset.count_le_of_le k (Multiset.map_le_map (f := Prod.fst) hle)
        rw [Multiset.map_add, Multiset.count_add] at hcount
        have h1 : 1 ≤ (rb'.map Prod.fst).count k :=
          Multiset.count_pos.mpr (Multiset.mem_map.mpr ⟨kv, hkv, hkey⟩)
        have h2 : 1 ≤ (rb.map Prod.fst).count k :=
          Multiset.count_pos.mpr (Multiset.mem_map.mpr ⟨(k, v), hresp, rfl⟩)
        have : ((cqConsumed (responses i) (dec i)).map Prod.fst).count k
            = ((batchCuts (responses i) 0 (dec i)).sum.map Prod.fst).count k := rfl
        omega
      -- the register tracks `k`'s metadata exactly while `k` is unanswered
      have hkeep : ∀ n, n ≤ u →
          (rem i n).filter (fun km => km.1 = k)
            = (((metadata i).take n).sum).filter (fun km => km.1 = k) := by
        intro n hn
        induction n with
        | zero => rw [hrem_zero i]; rfl
        | succ n ih =>
          have ih := ih (Nat.le_of_succ_le hn)
          -- tick `n` is realized on both inputs (tick `u` is)
          obtain ⟨rbn, hrbn⟩ : ∃ rbn, (batchCuts (responses i) 0 (dec i))[n]? = some rbn :=
            ⟨_, List.getElem?_eq_getElem (Nat.lt_of_lt_of_le (Nat.lt_of_succ_le hn)
              (Nat.le_of_lt (Trace.read_lt hrb)))⟩
          obtain ⟨mbn, hmbn⟩ : ∃ mbn, (metadata i)[n]? = some mbn :=
            ⟨_, List.getElem?_eq_getElem (Nat.lt_of_lt_of_le (Nat.lt_of_succ_le hn)
              (Nat.le_of_lt (Trace.read_lt hmb)))⟩
          rw [hrem_window i hrbn hmbn, List.take_add_one, List.sum_append, hmbn]
          simp only [Option.toList_some, List.sum_singleton]
          rw [Multiset.filter_filter, Multiset.filter_add, Multiset.filter_add, ← ih]
          -- the tick answered no `k`: the `k`-metadata passes the anti-join
          have hpass : ∀ s : Multiset (K × M),
              s.filter (fun km => km.1 = k ∧ ∀ kv ∈ rbn, kv.1 ≠ km.1)
                = s.filter (fun km => km.1 = k) := by
            intro s
            refine Multiset.filter_congr fun km _ => ?_
            constructor
            · exact fun h => h.1
            · intro h
              refine ⟨h, fun kv hkv heq => ?_⟩
              exact hno n (Nat.lt_of_succ_le hn) rbn hrbn kv hkv (heq.trans h)
          rw [hpass, hpass]
      -- at tick `u`, `k`'s metadata is in the window and `k` answers: joined
      have hu := hkeep u (le_refl u)
      have hmd' : ((k, m) : K × M) ∈ rem i u + mb := by
        rw [List.take_add_one, List.sum_append, hmb] at hmd
        simp only [Option.toList_some, List.sum_singleton] at hmd
        rcases Multiset.mem_add.mp hmd with h | h
        · have h' : ((k, m) : K × M) ∈ ((metadata i).take u).sum.filter (fun km => km.1 = k) :=
            Multiset.mem_filter.mpr ⟨h, rfl⟩
          rw [← hu] at h'
          exact Multiset.mem_add.mpr (Or.inl (Multiset.mem_of_le (Multiset.filter_le _ _) h'))
        · exact Multiset.mem_add.mpr (Or.inr h)
      -- the tick's join holds the pair
      obtain ⟨e, he⟩ : ∃ e, (joined i)[u]? = some e :=
        ⟨_, (hjoined_at i u _).mpr ⟨rb, mb, hrb, hmb, rfl⟩⟩
      refine mem_list_sum.mpr ⟨e, Trace.mem_of_read he, ?_⟩
      obtain ⟨rb', mb', hrb', hmb', rfl⟩ := (hjoined_at i u e).mp he
      obtain rfl := Trace.read_inj hrb hrb'
      obtain rfl := Trace.read_inj hmb hmb'
      simp only [joined_step, den, multisetJoin]
      refine Multiset.mem_map.mpr ⟨(k, (m, v)), ?_, rfl⟩
      refine Multiset.mem_bind.mpr ⟨(k, m), hmd', ?_⟩
      exact Multiset.mem_map.mpr ⟨(k, v), Multiset.mem_filter.mpr ⟨hresp, rfl⟩, rfl⟩

/-! ## Executable smoke tests (mirroring request_response.rs's) -/

#nondet_census join_responses (nondets := 1) (scheds := 0) (fuels := 0)

private abbrev jrOneLoc : Unit → Nat := fun _ => 1

-- basic join: metadata at tick 0, response at tick 1
#guard (join_responses (Values Unit jrOneLoc) ()
    (fun _ => ({(1, "resp")} : Multiset (Nat × String)))
    (fun _ => [({(1, 42)} : Multiset (Nat × Int)), 0])
    (fun _ => [0, {(1, "resp")}])) 0
  = ({(1, (42, "resp"))} : Multiset (Nat × (Int × String)))
-- metadata persists: generated two ticks before the response
#guard (join_responses (Values Unit jrOneLoc) ()
    (fun _ => ({(1, "resp")} : Multiset (Nat × String)))
    (fun _ => [({(1, 42)} : Multiset (Nat × Int)), 0, 0])
    (fun _ => [0, 0, {(1, "resp")}])) 0
  = ({(1, (42, "resp"))} : Multiset (Nat × (Int × String)))
-- no metadata, no join
#guard (join_responses (Values Unit jrOneLoc) ()
    (fun _ => ({(1, "resp")} : Multiset (Nat × String)))
    (fun _ => [(0 : Multiset (Nat × Int)), 0])
    (fun _ => [0, {(1, "resp")}])) 0
  = (0 : Multiset (Nat × (Int × String)))

end Hydro
