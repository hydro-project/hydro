import HydroV2.MonoRel
import HydroV2.Paxos.Types

/-!
# `p_ballot_calc` (paxos.rs:348–412)

The program is written **once**, SPMD over the proposer cluster: no `me`
parameter (closures take the member id — `q!` capturing
`CLUSTER_SELF_ID`), and the `Monotonic` bound on the ballot leg rides
the signature exactly like Rust's `Optional<Ballot, Tick, Monotonic>`.

Two artifacts about the same single body:
- `p_ballot_calc` — the polymorphic program (its Flo monotonicity is
  available on demand by instantiating the same text at `MonoRel` — no
  induction, no per-program content);
- `p_ballot_calc_ensures` — the module contract, proved over the
  `Values` denotation (the vocabulary ghost proofs and executable
  `#guard`s share).
-/

namespace HydroV2

variable {L : Type} {mem : L → Nat}

/-- The jump closure (paxos.rs:365–379); `me` is the captured
`CLUSTER_SELF_ID`. Named at file level so the contract proof below can
speak about it. -/
def pbcJump {nP : Nat} (me : Fin nP) (ballot_num : Nat)
    (received_max_ballot : Option (Ballot nP)) : Nat :=
  match received_max_ballot with
  | some rm =>
    if (Ballot.mk ballot_num me).blt rm then rm.num + 1 else ballot_num
  | none => ballot_num

/-- What `p_ballot_calc` **ensures** (per member of the cluster), over
the `Values` denotation. Value-order ascent of the ballot leg is *not* a
field: it rides the output type (`.1 i` is a `MonoTrace` — consumers
dot-access `.ascending`). -/
structure PBCEnsures (ℓ : L)
    (out : TickV (mem ℓ) (Ballot (mem ℓ))
        (.monotonic (Ballot.numVO (nP := mem ℓ)))
      × TickV (mem ℓ) Bool .unbounded) : Prop where
  /-- Ballots are owned by the emitting member. -/
  own : ∀ (i : Fin (mem ℓ)), ∀ b ∈ (out.1 i).vals,
    (b : Ballot (mem ℓ)).proposerId = i
  /-- `p_has_largest_ballot` is identically `true` on realized ticks: the
  zip pairs each view with the ballot computed *from that view*, and the
  jump always overtakes (paxos.rs:386–390). -/
  hasLargest_true : ∀ (i : Fin (mem ℓ)), ∀ g ∈ out.2 i, g = true



/-- **paxos.rs:348–412 `p_ballot_calc`**, over the proposer cluster `ℓ`.
Ballots are indexed by the cluster size (proposer ids are member ids). -/
def p_ballot_calc (H : HydroSem L mem) (ℓ : L)
    (received : H.TickSingleton ℓ (Option (Ballot (mem ℓ))) .unbounded) :
    {out : H.TickSingleton ℓ (Ballot (mem ℓ))
        (.monotonic (Ballot.numVO (nP := mem ℓ)))
      × H.TickSingleton ℓ Bool .unbounded //
      -- the colocated contract: *if* the instantiation is the `Values`
      -- denotation, the output satisfies `PBCEnsures` (proof over the
      -- body's own `let`s; ghost — erased at runtime)
      ∀ hv : H = Values L mem,
        match H, hv, out with
        | _, rfl, o => PBCEnsures ℓ o} :=
  -- let mut p_ballot_num = use::state(|l| l.singleton(q!(0)));
  -- p_ballot_num = p_received_max_ballot.zip(p_ballot_num).map(q!(jump))
  --   — with the `monotonic =` obligation inline at the fold
  let p_ballot_num := H.fold_across_ticks_monotone ValueOrder.nat pbcJump 0
    (fun me n rm => by
      show n ≤ pbcJump me n rm
      cases rm with
      | none => exact Nat.le_refl n
      | some b =>
        show n ≤ if (Ballot.mk n me).blt b then b.num + 1 else n
        by_cases h : (Ballot.mk n me).blt b
        · simp only [h, if_true]
          have h' := Ballot.blt_iff.mp h
          simp only [Ballot.num_mk, Ballot.proposerId_mk] at h'
          rcases h' with h' | ⟨h', _⟩ <;> omega
        · simp [h])
    received
  -- let p_ballot = p_ballot_num.map(|num| Ballot { num, CLUSTER_SELF_ID })
  --   [order-preserving]
  let p_ballot := H.mapMonotone Ballot.numVO p_ballot_num
    (fun me num => Ballot.mk num me) (fun _me {_a _b} h => h)
  -- let p_has_largest_ballot = received.zip(p_ballot)
  --   .map(q!(|(rm, cur)| rm <= Some(cur)))  (paxos.rs:386–390)
  let p_has_largest_ballot :=
    H.mapTick (H.zipTick received (H.forgetBound p_ballot))
      (fun _me rc => Ballot.optLe rc.1 rc.2)
  ⟨(p_ballot, p_has_largest_ballot), by
    intro hv
    subst hv
    constructor
    · -- ownership: every emitted ballot is `Ballot.mk _ i`
      intro i b hb
      obtain ⟨num, -, rfl⟩ := List.mem_map.mp hb
      rfl
    · -- the overtake: the zip pairs each view with the ballot computed
      -- from that view
      intro i g hg

      have hg' : g ∈ (Trace.zip (received i)
          ((foldAcrossTicksTrace (pbcJump i) 0 (received i)).map
            (fun num => Ballot.mk num i))).map
          (fun rc => Ballot.optLe rc.1 rc.2) := hg
      obtain ⟨t, ht, hgt⟩ := List.mem_iff_getElem.mp hg'
      subst hgt
      have hlen : ((foldAcrossTicksTrace (pbcJump i) 0 (received i)).map
          (fun num => Ballot.mk num i)).length
          = (received i).length := by
        rw [List.length_map, foldAcrossTicksTrace_length]
      have htv : t < (received i).length := by
        have h0 := ht
        simp only [Trace.zip, List.length_map, List.length_zip] at h0
        omega
      have htb : t < ((foldAcrossTicksTrace (pbcJump i) 0 (received i)).map
          (fun num => Ballot.mk num i)).length := by
        rw [hlen]
        exact htv
      have hzip : t < (Trace.zip (received i)
          ((foldAcrossTicksTrace (pbcJump i) 0 (received i)).map
            (fun num => Ballot.mk num i))).length := by
        simp only [Trace.zip, List.length_zip]
        rw [hlen]
        omega
      rw [List.getElem_map]
      have hpair : (Trace.zip (received i)
          ((foldAcrossTicksTrace (pbcJump i) 0 (received i)).map
            (fun num => Ballot.mk num i)))[t]'hzip
          = ((received i)[t]'htv,
             ((foldAcrossTicksTrace (pbcJump i) 0 (received i)).map
               (fun num => Ballot.mk num i))[t]'htb) := by
        simp only [Trace.zip]
        exact List.getElem_zip ..
      rw [hpair]
      -- the ballot at `t` is the fold of the view through `t`
      have hget : ((foldAcrossTicksTrace (pbcJump i) 0 (received i)).map
          (fun num => Ballot.mk num i))[t]'htb
          = Ballot.mk (((received i).take (t + 1)).foldl (pbcJump i) 0) i := by
        rw [List.getElem_map, foldAcrossTicksTrace_getElem]
      rw [hget]
      -- the overtake: after the jump, our ballot is never behind the view
      have hfold : ((received i).take (t + 1)).foldl (pbcJump i) 0
          = pbcJump i (((received i).take t).foldl (pbcJump i) 0)
              ((received i)[t]'htv) := by
        rw [List.take_succ, List.foldl_append]
        rw [List.getElem?_eq_getElem htv]
        rfl
      rw [hfold]
      generalize ((received i).take t).foldl (pbcJump i) 0 = n
      show Ballot.optLe ((received i)[t]'htv)
        (Ballot.mk (pbcJump i n ((received i)[t]'htv)) i) = true
      cases hrm : (received i)[t]'htv with
      | none => rfl
      | some b =>
        show Ballot.optLe (some b)
          (Ballot.mk (if (Ballot.mk n i).blt b then b.num + 1 else n) i)
            = true
        by_cases h : (Ballot.mk n i).blt b
        · simp only [h, if_true]
          exact Ballot.ble_iff.mpr (Or.inl (Ballot.blt_iff.mpr
            (Or.inl (Nat.lt_succ_self _))))
        · simp only [h, if_false]
          have h' := fun hc => h (Ballot.blt_iff.mpr hc)
          refine Ballot.ble_iff.mpr ?_
          by_cases hnum : b.num = n
          · by_cases hid : b.proposerId = i
            · refine Or.inr ?_
              cases b with
              | mk bn bp =>
                simp only [Ballot.num_mk, Ballot.proposerId_mk] at hnum hid
                rw [hnum, hid]
                rfl
            · refine Or.inl (Ballot.blt_iff.mpr (Or.inr ⟨hnum, ?_⟩))
              rcases Nat.lt_trichotomy b.proposerId.val i.val
                with hp | hp | hp
              · exact hp
              · exact absurd (Fin.ext hp) hid
              · exact absurd (Or.inr ⟨hnum.symm, hp⟩) h'
          · refine Or.inl (Ballot.blt_iff.mpr (Or.inl ?_))
            rcases Nat.lt_trichotomy b.num n with hlt | heq | hgt
            · exact hlt
            · exact absurd heq hnum
            · exact absurd (Or.inl hgt) h'

⟩

/-! ## Executable smoke test (`@Values` evaluates) -/

/-- A two-proposer cluster at some location. -/
abbrev twoProp : Unit → Nat := fun _ => 2

-- Member 0 sees `[none, some (5, member 1)]`: tick 0 keeps ballot
-- `(0, 0)`; tick 1 jumps over `(5, 1)` to `(6, 0)`.
#guard (((p_ballot_calc (Values Unit twoProp) ()
    (fun _i => [none, some (Ballot.mk 5 1)])).val.1 0).vals.map
      (fun b => b.num)) = [0, 6]

-- Member 1 also jumps to `6` (same num, lower id ⇒ `blt`).
#guard (((p_ballot_calc (Values Unit twoProp) ()
    (fun _i => [none, some (Ballot.mk 5 1)])).val.1 1).vals.map
      (fun b => b.num)) = [0, 6]

-- `p_has_largest_ballot` is identically `true` (the ensures field,
-- observed executably).
#guard ((p_ballot_calc (Values Unit twoProp) ()
    (fun _i => [none, some (Ballot.mk 5 1)])).val.2 0) = [true, true]

end HydroV2
