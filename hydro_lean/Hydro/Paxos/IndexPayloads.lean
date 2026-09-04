import Hydro.MonoRel
import Hydro.Paxos.Types
import Hydro.HydroDef
import Hydro.HydroTick

/-!
# `index_payloads` (paxos.rs:776–806)

Assign consecutive slots to the leader's payload batch: the base slot is
`p_max_slot + 1` when phase-1 reconciliation produced one (a fresh
leader continues after the recovered log), else the cross-tick
`next_slot` state; the batch is enumerated from the base and the state
advances by the batch size. The Rust `sliced!` block (`use::state`),
line for line, as a `tick` block over the in-tick operators. The
contract (`IPEnsures`) is stated on the output wire per tick; its proof
runs INLINE along the program, reading the block's register through the
construct's `_reg` and the body through its generated step under the
`den` simp set — no pure twin of the body (FINDINGS D64 E6/E7).

Before the program: only its contract face. **The program starts at
`hydro def index_payloads`.**
-/

namespace Hydro

variable {L : Type} {mem : L → Nat} {P : Type} [DecidableEq P]

/-! ## Prerequisites for the contract face -/

/-- What `index_payloads` **ensures**, over the `Values` denotation —
stated on the output wire, per tick (`[t]?` reads): slots are
consecutive from a base pinned by a rebase; between rebases the register
only climbs, so later ticks sit above earlier ones and above the last
recovered max. -/
structure IPEnsures (prop : L)
    (ms : TickV (mem prop) (Option Nat))
    (bs : Fin (mem prop) → Trace (List P))
    (out : Fin (mem prop) → Trace (List (Nat × P))) : Prop where
  /-- **Rebase dominance**: a tick's recovered max slot sits below every
  slot indexed from that tick until the next rebase. -/
  rebase_dominates : ∀ (i : Fin (mem prop)) {u v : Nat}, u ≤ v →
    ∀ {m : Nat}, (ms i)[u]? = some (some m) →
    (∀ w, u < w → w ≤ v → (ms i)[w]? = some none) →
    ∀ {ev : List (Nat × P)}, (out i)[v]? = some ev →
    ∀ s ∈ ev.map Prod.fst, m < s
  /-- **Slot domination**: with no rebase in `(u, v]`, tick `v`'s slots are
  all above tick `u`'s. -/
  slots_dominate : ∀ (i : Fin (mem prop)) {u v : Nat}, u < v →
    (∀ w, u < w → w ≤ v → (ms i)[w]? = some none) →
    ∀ {eu ev : List (Nat × P)}, (out i)[u]? = some eu → (out i)[v]? = some ev →
    ∀ s ∈ eu.map Prod.fst, ∀ s' ∈ ev.map Prod.fst, s < s'
  /-- **Within a tick, slots are distinct** (consecutive from the base). -/
  slots_nodup : ∀ (i : Fin (mem prop)) {t : Nat} {e : List (Nat × P)},
    (out i)[t]? = some e → (e.map Prod.fst).Nodup

/-! ## The program -/

/-- **paxos.rs:776–806 `index_payloads`**. -/
hydro def index_payloads (H : HydroSem L mem) (prop : L)
    (p_max_slot : H.Ticked prop (Option Nat))
    (c_to_proposers : H.TickStream prop P .totalOrder .exactlyOnce) :
    H.TickStream prop (Nat × P) .totalOrder .exactlyOnce
  ensures out => IPEnsures prop p_max_slot c_to_proposers out :=
  -- sliced! {
  --   let mut next_slot = use::state(|l| l.singleton(q!(0)));
  --   let updated_max_slot = use::atomic(p_max_slot.latest_atomic(), …);
  --   let payload_batch = use::atomic(c_to_proposers.all_ticks_atomic(), …);
  tick (state next_slot : Nat := 0)
      (input updated_max_slot := p_max_slot)
      (input payload_batch := c_to_proposers) :=
    -- let next_slot_after_reconciling_p1bs = updated_max_slot.map(q!(|s| s + 1));
    let next_slot_after_reconciling_p1bs := H.boMap updated_max_slot (· + 1)
    -- let base_slot = next_slot_after_reconciling_p1bs.unwrap_or(next_slot);
    let base_slot := H.boUnwrapOr next_slot_after_reconciling_p1bs next_slot
    -- let indexed_payloads = payload_batch.enumerate().cross_singleton(base_slot.clone())
    --   .map(q!(|((index, payload), base_slot)| (base_slot + index, payload)));
    let indexed_payloads := H.bmap
      (H.bcrossSingleton (H.benumerate payload_batch) base_slot)
      (fun ((index, payload), base_slot) => (base_slot + index, payload))
    -- let num_payloads = indexed_payloads.clone().count();
    let num_payloads := H.bcount indexed_payloads
    -- next_slot = num_payloads.zip(base_slot).map(q!(|(num_payloads, base_slot)| base_slot + num_payloads));
    rebind (next_slot := H.bsMap (H.bsZip num_payloads base_slot)
      (fun (num_payloads, base_slot) => base_slot + num_payloads))
    -- yield_atomic(indexed_payloads)
    yield (indexed_payloads := indexed_payloads);
  -- }
  -- **the register, named** (`reg i n` = `next_slot` before tick `n`):
  -- read at a tick, stepped, frozen once the payload input ends — the
  -- construct's `_reg`, the one place the block's fold is read
  ghost obtain ⟨reg, -, hreg_at, hreg_succ, -, hreg_stall⟩ := hindexed_payloads_reg
  -- **a tick's slots**: the next `batch.length` naturals from the tick's
  -- base — the rebased max (if any), else the register
  ghost have hslots : ∀ (i : Fin (mem prop)) {n : Nat} {e : List (Nat × P)},
      (indexed_payloads i)[n]? = some e →
      ∃ (m : Option Nat) (b : List P),
        (p_max_slot i)[n]? = some m ∧ (c_to_proposers i)[n]? = some b
        ∧ e.map Prod.fst = List.range' ((m.map (· + 1)).getD (reg i n)) b.length :=
    fun i {n e} he => by
    obtain ⟨m, b, hm, hb, rfl⟩ := (hreg_at i n e).mp he
    refine ⟨m, b, hm, hb, ?_⟩
    simp only [indexed_payloads_step, den]
    rw [List.map_map, List.map_map]
    exact listEnumerate_indices_add _ b
  -- **the register advances past the tick's slots**: base plus batch size
  ghost have hadvance : ∀ (i : Fin (mem prop)) {n : Nat} {m : Option Nat} {b : List P},
      (p_max_slot i)[n]? = some m → (c_to_proposers i)[n]? = some b →
      reg i (n + 1) = (m.map (· + 1)).getD (reg i n) + b.length :=
    fun i {n m b} hm hb => by
    rw [hreg_succ i n m b hm hb]
    simp only [indexed_payloads_step, den, listEnumerate, List.length_map, List.length_zipIdx]
  -- **the register only climbs while no rebase happens**
  ghost have hreg_climb : ∀ (i : Fin (mem prop)) {u k : Nat},
      (∀ w, u ≤ w → w < u + k → (p_max_slot i)[w]? = some none) →
      reg i u ≤ reg i (u + k) := fun i {u k} hnone => by
    induction k with
    | zero => exact le_refl _
    | succ k ih =>
      have hle := ih (fun w hw hw' => hnone w hw (by omega))
      rw [show u + (k + 1) = u + k + 1 by omega]
      cases hb : (c_to_proposers i)[u + k]? with
      | none => rw [hreg_stall i (u + k) hb]; exact hle
      | some b =>
        rw [hadvance i (hnone (u + k) (by omega) (by omega)) hb]
        show reg i u ≤ reg i (u + k) + b.length
        omega
  indexed_payloads
  prove
    rebase_dominates := fun i {u v} huv {m} hmu hnone {ev} hev s hs => by
      -- the rebase sets the base to `m + 1`; the register climbs from there
      obtain ⟨mv, bv, hmv, hbv, hsl⟩ := hslots i hev
      rw [hsl, List.mem_range'_1] at hs
      obtain ⟨hlo, -⟩ := hs
      rcases Nat.lt_or_eq_of_le huv with hlt | rfl
      · -- `v` is not the rebase tick: its base is the climbed register
        have hmv' : mv = none := Option.some.inj (hmv.symm.trans (hnone v hlt (le_refl _)))
        subst hmv'
        -- the register after the rebase tick is above `m`
        obtain ⟨bu, hbu⟩ : ∃ bu, (c_to_proposers i)[u]? = some bu := by
          -- `u`'s payload batch exists since `v > u` does
          obtain ⟨hv', -⟩ := List.getElem?_eq_some_iff.mp hbv
          exact ⟨_, List.getElem?_eq_getElem (Nat.lt_trans hlt hv')⟩
        have h1 : m + 1 ≤ reg i (u + 1) := by
          rw [hadvance i hmu hbu]
          show m + 1 ≤ m + 1 + bu.length
          omega
        have h2 : reg i (u + 1) ≤ reg i v := by
          have := hreg_climb i (u := u + 1) (k := v - (u + 1))
            (fun w hw hw' => hnone w (by omega) (by omega))
          rwa [show u + 1 + (v - (u + 1)) = v by omega] at this
        change reg i v ≤ s at hlo
        omega
      · -- `v` is the rebase tick itself
        have hmv' : mv = some m := Option.some.inj (hmv.symm.trans hmu)
        subst hmv'
        change m + 1 ≤ s at hlo
        omega,
    slots_dominate := fun i {u v} huv hnone {eu ev} heu hev s hs s' hs' => by
      obtain ⟨mu, bu, hmu, hbu, hslu⟩ := hslots i heu
      obtain ⟨mv, bv, hmv, hbv, hslv⟩ := hslots i hev
      have hmv' : mv = none := Option.some.inj (hmv.symm.trans (hnone v huv (le_refl _)))
      subst hmv'
      rw [hslu, List.mem_range'_1] at hs
      rw [hslv, List.mem_range'_1] at hs'
      obtain ⟨-, hhi⟩ := hs
      obtain ⟨hlo', -⟩ := hs'
      change reg i v ≤ s' at hlo'
      -- every slot at `u` is below the register after `u`, which climbs to
      -- the register before `v`, which is `v`'s base
      have h2 : reg i (u + 1) ≤ reg i v := by
        have := hreg_climb i (u := u + 1) (k := v - (u + 1))
          (fun w hw hw' => hnone w (by omega) (by omega))
        rwa [show u + 1 + (v - (u + 1)) = v by omega] at this
      have h1 := hadvance i hmu hbu
      cases mu with
      | none =>
        change s < reg i u + bu.length at hhi
        change reg i (u + 1) = reg i u + bu.length at h1
        omega
      | some m₀ =>
        change s < m₀ + 1 + bu.length at hhi
        change reg i (u + 1) = m₀ + 1 + bu.length at h1
        omega,
    slots_nodup := fun i {t e} he => by
      obtain ⟨m, b, -, -, hsl⟩ := hslots i he
      rw [hsl]
      exact List.nodup_range' 1

/-! ## Executable smoke tests -/

-- A stable leader: consecutive slots across ticks from the state.
#guard (index_payloads (Values PaxLoc (paxMem 1 1)) .prop
    (fun _ => [none, none])
    (fun _ => [[10, 11], [12]] : Fin 1 → Trace (List Nat))) 0
  = [[(0, 10), (1, 11)], [(2, 12)]]

-- A fresh leader reconciles: the recovered max slot rebases indexing.
#guard (index_payloads (Values PaxLoc (paxMem 1 1)) .prop
    (fun _ => [some 5, none])
    (fun _ => [[10, 11], [12]] : Fin 1 → Trace (List Nat))) 0
  = [[(6, 10), (7, 11)], [(8, 12)]]

end Hydro
