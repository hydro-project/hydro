import Hydro.MonoRel
import Hydro.Paxos.Types
import Hydro.HydroTick

/-!
# `p_ballot_calc` (paxos.rs:348–412)

The program is written **once**, SPMD over the proposer cluster: no `me`
parameter (closures take the member id — `q!` capturing
`CLUSTER_SELF_ID`), and the `Monotonic` bound on the ballot leg rides
the signature exactly like Rust's `Optional<Ballot, Tick, Monotonic>`.
Its Flo monotonicity is available on demand by instantiating the same
text at `MonoRel`; the module contract (`PBCEnsures`) is proved INLINE
over the `Values` denotation, reading the `p_ballot_num` register through
the construct's `_reg` and the jump closure through the generated step
under `den` (FINDINGS D64).

Before the program: only its contract face. **The program starts at
`hydro def p_ballot_calc`.**
-/

namespace Hydro

variable {L : Type} {mem : L → Nat}

/-! ## Prerequisites for the contract face -/

/-- What `p_ballot_calc` **ensures** (per member of the cluster), over
the `Values` denotation. -/
structure PBCEnsures (ℓ : L)
    (out : TickV (mem ℓ) (Ballot (mem ℓ)) × TickV (mem ℓ) Bool) : Prop where
  /-- Ballots are owned by the emitting member. -/
  own : ∀ (i : Fin (mem ℓ)), ∀ b ∈ out.1 i,
    (b : Ballot (mem ℓ)).proposerId = i
  /-- **Ascent**: ballot numbers only grow along the member's ticks (the
  `use::state` register jumps past every view it consumes) — a contract
  fact, since a tick singleton is always `Bounded` in Rust. -/
  mono : ∀ (i : Fin (mem ℓ)), Ascending Ballot.numVO (out.1 i)
  /-- `p_has_largest_ballot` is identically `true` on realized ticks: the
  zip pairs each view with the ballot computed *from that view*, and the
  jump always overtakes (paxos.rs:386–390). -/
  hasLargest_true : ∀ (i : Fin (mem ℓ)), ∀ g ∈ out.2 i, g = true

/-! ## The program -/

/-- **paxos.rs:348–412 `p_ballot_calc`**, over the proposer cluster `ℓ`.
Ballots are indexed by the cluster size (proposer ids are member ids). -/
hydro def p_ballot_calc (H : HydroSem L mem) (ℓ : L)
    (received : H.Ticked ℓ (Option (Ballot (mem ℓ)))) :
    (H.Ticked ℓ (Ballot (mem ℓ))
      × H.Ticked ℓ Bool)
  ensures out => PBCEnsures ℓ out :=
  -- let (p_ballot, p_has_largest_ballot) = sliced! {
  --   let p_received_max_ballot = use::atomic(…, nondet!(/** up to date with tick input */));
  --   let mut p_ballot_num = use::state(|l| l.singleton(q!(0)));
  tick (state p_ballot_num : Nat := 0)
      (input p_received_max_ballot := received) :=
    -- p_ballot_num = p_received_max_ballot.clone().zip(p_ballot_num)
    --   .map(q!(move |(received_max_ballot, ballot_num)| {
    --     if let Some(received_max_ballot) = received_max_ballot {
    --       if received_max_ballot > (Ballot { num: ballot_num, proposer_id: CLUSTER_SELF_ID }) {
    --         received_max_ballot.num + 1
    --       } else { ballot_num }
    --     } else { ballot_num }
    --   }));
    let p_ballot_num := H.bsMap (H.bsZip p_received_max_ballot p_ballot_num)
      (fun (received_max_ballot, ballot_num) =>
        match received_max_ballot with
        | some received_max_ballot =>
          if (Ballot.mk ballot_num me).blt received_max_ballot
          then received_max_ballot.num + 1 else ballot_num
        | none => ballot_num)
    -- let p_ballot = p_ballot_num.clone().map(q!(move |num| Ballot { num, proposer_id: CLUSTER_SELF_ID.clone() }));
    let p_ballot := H.bsMap p_ballot_num (fun num => Ballot.mk num me)
    -- let p_has_largest_ballot = p_received_max_ballot.zip(p_ballot.clone())
    --   .map(q!(|(received_max_ballot, cur_ballot)| received_max_ballot <= Some(cur_ballot)));
    let p_has_largest_ballot := H.bsMap (H.bsZip p_received_max_ballot p_ballot)
      (fun (received_max_ballot, cur_ballot) =>
        Ballot.optLe received_max_ballot cur_ballot)
    rebind (p_ballot_num := p_ballot_num)
    -- (yield_atomic(p_ballot), yield_atomic(p_has_largest_ballot))
    emit (p_ballot := p_ballot, p_has_largest_ballot := p_has_largest_ballot);
  -- };
  -- **the register, named** (`reg i n` = `p_ballot_num` before tick `n`):
  -- read on both legs, stepped, frozen once the views end — the
  -- construct's `_reg`, the one place the block's fold is read
  ghost obtain ⟨reg, -, hpb_at, hhl_at, hreg_succ, hreg_stall⟩ := hp_ballot_reg
  -- **the jump never lowers the number** (the register's ascent): the
  -- consumed view either raises it past itself or leaves it
  ghost have hjump_le : ∀ (i : Fin (mem ℓ)) (n : Nat) (rm : Option (Ballot (mem ℓ))),
      n ≤ @id Nat (p_ballot_step i n rm).1 := fun i n rm => by
    simp only [id, p_ballot_step, den]
    cases rm with
    | none => exact Nat.le_refl n
    | some b =>
      simp only []
      by_cases h : (Ballot.mk n i).blt b = true
      · rw [if_pos h]
        have h' := Ballot.blt_iff.mp h
        simp only [Ballot.num_mk, Ballot.proposerId_mk] at h'
        rcases h' with h' | ⟨h', _⟩ <;> omega
      · rw [if_neg h]
  -- **the emitted ballot carries the stepped number**, stamped with the member
  ghost have hballot : ∀ (i : Fin (mem ℓ)) (n : Nat) (rm : Option (Ballot (mem ℓ))),
      @id (Ballot (mem ℓ)) (p_ballot_step i n rm).2.1
        = Ballot.mk (@id Nat (p_ballot_step i n rm).1) i := fun i n rm => by
    simp only [id, p_ballot_step, den]
  -- **the jump overtakes** (paxos.rs:386–390): after consuming a view, our
  -- ballot is never behind it — THE protocol content of this module; the
  -- flag the block emits is identically `true`
  ghost have hovertake : ∀ (i : Fin (mem ℓ)) (n : Nat) (rm : Option (Ballot (mem ℓ))),
      @id Bool (p_ballot_step i n rm).2.2 = true := fun i n rm => by
    simp only [id, p_ballot_step, den]
    cases rm with
    | none => rfl
    | some b =>
      simp only []
      by_cases h : (Ballot.mk n i).blt b = true
      · rw [if_pos h]
        exact Ballot.ble_iff.mpr (Or.inl (Ballot.blt_iff.mpr (Or.inl (Nat.lt_succ_self _))))
      · rw [if_neg h]
        have h' := fun hc => h (Ballot.blt_iff.mpr hc)
        refine Ballot.ble_iff.mpr ?_
        by_cases hnum : b.num = n
        · by_cases hid : b.proposerId = i
          · refine Or.inr ?_
            cases b with
            | mk bn bp =>
              simp only [Ballot.num_mk, Ballot.proposerId_mk] at hnum hid
              rw [hnum, hid]
          · refine Or.inl (Ballot.blt_iff.mpr (Or.inr ⟨hnum, ?_⟩))
            rcases Nat.lt_trichotomy b.proposerId.val i.val with hp | hp | hp
            · exact hp
            · exact absurd (Fin.ext hp) hid
            · exact absurd (Or.inr ⟨hnum.symm, hp⟩) h'
        · refine Or.inl (Ballot.blt_iff.mpr (Or.inl ?_))
          rcases Nat.lt_trichotomy b.num n with hlt | heq | hgt
          · exact hlt
          · exact absurd heq hnum
          · exact absurd (Or.inl hgt) h'
  -- **the register only climbs**
  ghost have hreg_mono : ∀ (i : Fin (mem ℓ)) (n k : Nat), reg i n ≤ reg i (n + k) :=
    fun i n k => by
    induction k with
    | zero => exact Nat.le_refl _
    | succ k ih =>
      rw [show n + (k + 1) = n + k + 1 by omega]
      cases hrm : (received i)[n + k]? with
      | none => rw [hreg_stall i (n + k) hrm]; exact ih
      | some rm =>
        rw [hreg_succ i (n + k) rm hrm]
        exact le_trans ih (hjump_le i (reg i (n + k)) rm)
  (p_ballot, p_has_largest_ballot)
  prove
    own := fun i b hb => by
      obtain ⟨t, ht⟩ := List.mem_iff_getElem?.mp hb
      obtain ⟨rm, -, rfl⟩ := (hpb_at i t b).mp ht
      rw [show (p_ballot_step i (reg i t) rm).2.1
          = @id (Ballot (mem ℓ)) (p_ballot_step i (reg i t) rm).2.1 from rfl, hballot],
    mono := fun i {t t'} h ht' => by
      -- a tick's ballot number is the register after it; the register climbs
      obtain ⟨rm, hrm, hb⟩ := (hpb_at i t _).mp (List.getElem?_eq_getElem (Nat.lt_of_le_of_lt h ht'))
      obtain ⟨rm', hrm', hb'⟩ := (hpb_at i t' _).mp (List.getElem?_eq_getElem ht')
      show ((p_ballot i)[t]'_).num ≤ ((p_ballot i)[t']'ht').num
      rw [hb, hb', show (p_ballot_step i (reg i t) rm).2.1
          = @id (Ballot (mem ℓ)) (p_ballot_step i (reg i t) rm).2.1 from rfl,
        show (p_ballot_step i (reg i t') rm').2.1
          = @id (Ballot (mem ℓ)) (p_ballot_step i (reg i t') rm').2.1 from rfl,
        hballot, hballot, Ballot.num_mk, Ballot.num_mk,
        ← hreg_succ i t rm hrm, ← hreg_succ i t' rm' hrm']
      have := hreg_mono i (t + 1) (t' - t)
      rwa [show t + 1 + (t' - t) = t' + 1 by omega] at this,
    hasLargest_true := fun i g hg => by
      obtain ⟨t, ht⟩ := List.mem_iff_getElem?.mp hg
      obtain ⟨rm, -, rfl⟩ := (hhl_at i t g).mp ht
      exact hovertake i (reg i t) rm

/-! ## Executable smoke test (`@Values` evaluates) -/

/-- A two-proposer cluster at some location. -/
abbrev twoProp : Unit → Nat := fun _ => 2

-- Member 0 sees `[none, some (5, member 1)]`: tick 0 keeps ballot
-- `(0, 0)`; tick 1 jumps over `(5, 1)` to `(6, 0)`.
#guard (((p_ballot_calc (Values Unit twoProp) ()
    (fun _i => [none, some (Ballot.mk 5 1)])).1 0).map
      (fun b => b.num)) = [0, 6]

-- Member 1 also jumps to `6` (same num, lower id ⇒ `blt`).
#guard (((p_ballot_calc (Values Unit twoProp) ()
    (fun _i => [none, some (Ballot.mk 5 1)])).1 1).map
      (fun b => b.num)) = [0, 6]

-- `p_has_largest_ballot` is identically `true` (the ensures field,
-- observed executably).
#guard ((p_ballot_calc (Values Unit twoProp) ()
    (fun _i => [none, some (Ballot.mk 5 1)])).2 0) = [true, true]

end Hydro
