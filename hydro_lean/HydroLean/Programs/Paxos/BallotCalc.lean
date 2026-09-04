import HydroLean.Programs.Paxos.Types
import HydroLean.Hydro.TStream
import HydroLean.Hydro.Growth
import HydroLean.Hydro.MonoSing

/-!
# `p_ballot_calc` (paxos.rs:348–412) — module

Proposer logic to calculate the next ballot number, given the (snapshot view
of the) largest ballot received so far. In Rust this is its own function over
a `sliced!` block whose inputs are `use::atomic` (pinned to `proposer_tick`);
the *staleness* of the received-max view is chosen upstream, at the
`.snapshot(proposer_tick, nondet_leader)` call site (paxos.rs:287–294), so
this module is a pure, deterministic per-tick step — exactly as its Rust
signature (no `NonDet` parameter) claims.

Module spec (used by `leader_election`'s correctness):
- the ballot number never decreases — **carried by the output type**
  (`MonoSing ballotNumVO`, the Rust `Monotonic` singleton bound; consumers
  project `.ascending`);
- `ballot_beats_view`: the produced ballot is never behind the view it was
  given (`p_has_largest_ballot` is `true` whenever the view is not ahead of
  a *fresher* ballot owned by another proposer — with self-owned nums the
  jump always overtakes, `ballot_overtakes`).
-/

namespace HydroLean.Programs.Paxos

variable {nP : Nat}

/-! ## `p_ballot_calc`, transcribed across ticks (paxos.rs:348–412)

The Rust body over the located surface (`Hydro/TStream.lean`): the input is
the per-tick snapshot view of `p_received_max_ballot` (the staleness choice
was made upstream, at the `.snapshot(proposer_tick, nondet_leader)` site —
this function has no `NonDet` parameter, exactly like its Rust signature);
`use::state` is the state scan. -/

/-- The self-owned ballot value order: growth of the `num` field (the order
`p_ballot`'s `Monotonic` bound is about — proposer id is fixed). -/
def ballotNumVO {nP : Nat} : HydroLean.Hydro.ValueOrder (Ballot nP) :=
  ⟨fun a b => a.num ≤ b.num, fun _ => Nat.le_refl _, Nat.le_trans⟩

open HydroLean.Hydro

namespace p_ballot_calc

/-- What `p_ballot_calc` **ensures** (the Rust-visible guarantees): the
output pair (`Monotonic` ballot leg, has-largest wire) against the
received-max input `v`. -/
structure Ensures (me : Fin nP) (v : TSing (Option (Ballot nP)))
    (out : MonoSing (ballotNumVO (nP := nP)) × TSing Bool) : Prop where
  /-- Ballots are owned by construction. -/
  own : ∀ b ∈ out.1.vals, (b : Ballot nP).proposerId = me
  /-- `p_has_largest_ballot` is identically `true` on realized ticks: the
  zip pairs each view with the ballot computed *from that view*, and the
  jump always overtakes. Rust-visible face (paxos.rs:386–390). -/
  hasLargest_true : ∀ g ∈ out.2, g = true

end p_ballot_calc

/-- **paxos.rs:348–412 `p_ballot_calc`** — the single verified artifact:
the body (one `let` per Rust binding), its prefix-monotonicity (`.mono`,
by the `→ₘ` composition), and its guarantees (`.ensures`, the
`p_ballot_calc.Ensures` record) in one definition. The ballot leg's
`MonoSing` output is its Rust `Monotonic` bound — the `use::state` fold
carries its `monotone =` justification (`pBallotCalc_mono`) and consumers
read cross-tick growth off the type (`.ascending`). -/
def p_ballot_calc (me : Fin nP) :
    Verified (TSing (Option (Ballot nP)))
      (MonoSing (ballotNumVO (nP := nP)) × TSing Bool)
      (p_ballot_calc.Ensures me) :=
  -- the jump closure (paxos.rs:365–379): if the received max beats
  -- `Ballot { num, proposer_id: me }`, jump to `received.num + 1`
  let jump := fun (ballot_num : Nat)
      (received_max_ballot : Option (Ballot nP)) =>
    match received_max_ballot with
    | some rm =>
      if (Ballot.mk ballot_num me).blt rm then rm.num + 1 else ballot_num
    | none => ballot_num
  -- let mut p_ballot_num = use::state(|l| l.singleton(q!(0)));
  -- p_ballot_num = p_received_max_ballot.zip(p_ballot_num).map(q!(jump))
  --   — with the `monotonic =` obligation inline at the fold
  let p_ballot_num := fold_monotonicM ValueOrder.nat jump 0
    (fun n rm => by
      show n ≤ jump n rm
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
  -- let p_ballot = p_ballot_num.map(|num| Ballot { num, CLUSTER_SELF_ID })
  --   [order-preserving]
  let p_ballot := p_ballot_num.mapWire (fun num => Ballot.mk num me)
    (fun h => h)
  -- let p_has_largest_ballot = received.zip(p_ballot)
  --   .map(q!(|(rm, cur)| rm <= Some(cur)))  (paxos.rs:386–390)
  let p_has_largest_ballot := (MonoMap.id.zip p_ballot.vals).map
    (fun rc => Ballot.optLe rc.1 rc.2)
  Verified.ofMono (MonoMap.pair p_ballot p_has_largest_ballot)
    (fun v =>
    let bx := (p_ballot.f v).vals
    let glx := p_has_largest_ballot.f v
    -- ghost: ownership, at the binding
    have hown : ∀ b ∈ bx, (b : Ballot nP).proposerId = me := fun b hb => by
      obtain ⟨num, -, rfl⟩ := List.mem_map.mp hb
      rfl
    -- ghost: the zip pairs each view with the ballot computed FROM that
    -- view, and the jump always overtakes
    have hgl : ∀ g ∈ glx, g = true := fun g hg => by
      obtain ⟨t, ht, hgt⟩ := List.mem_iff_getElem.mp hg
      subst hgt
      have hlen : bx.length = v.length := by
        show ((scanSt jump 0 v).map _).length = _
        rw [List.length_map, scanSt_length]
      have hlen' : glx.length ≤ v.length := by
        show ((TSing.zip _ _).map _).length ≤ _
        simp only [TSing.zip, List.length_map, List.length_zip]
        exact Nat.min_le_left _ _
      have htv : t < v.length := Nat.lt_of_lt_of_le ht hlen'
      have htb : t < bx.length := by
        rw [hlen]
        exact htv
      have hzip : t < (TSing.zip v bx).length := by
        simp only [TSing.zip, List.length_zip]
        rw [hlen]
        omega
      have ht' : t < ((TSing.zip v bx).map
        (fun rc => Ballot.optLe rc.1 rc.2)).length := ht
      show ((TSing.zip v bx).map
        (fun rc => Ballot.optLe rc.1 rc.2))[t]'ht' = true
      rw [List.getElem_map]
      have hpair : (TSing.zip v bx)[t]'hzip = (v[t]'htv, bx[t]'htb) := by
        simp only [TSing.zip]
        exact List.getElem_zip ..
      rw [hpair]
      -- the ballot at `t` is the fold of the view through `t`
      have htb' : t < ((scanSt jump 0 v).map
          (fun num => Ballot.mk num me)).length := htb
      have hget : bx[t]'htb
          = Ballot.mk ((v.take (t + 1)).foldl jump 0) me := by
        show ((scanSt jump 0 v).map (fun num => Ballot.mk num me))[t]'htb'
          = _
        rw [List.getElem_map, scanSt_getElem]
      rw [hget]
      -- the overtake: after the jump, our ballot is never behind the view
      have hfold : (v.take (t + 1)).foldl jump 0
          = jump ((v.take t).foldl jump 0) (v[t]'htv) := by
        rw [List.take_succ, List.foldl_append]
        rw [List.getElem?_eq_getElem htv]
        rfl
      rw [hfold]
      generalize (v.take t).foldl jump 0 = n
      show Ballot.optLe (v[t]'htv) (Ballot.mk (jump n (v[t]'htv)) me) = true
      cases hrm : v[t]'htv with
      | none => rfl
      | some b =>
        show Ballot.optLe (some b)
          (Ballot.mk (if (Ballot.mk n me).blt b then b.num + 1 else n) me)
            = true
        by_cases h : (Ballot.mk n me).blt b
        · simp only [h, if_true]
          exact Ballot.ble_iff.mpr (Or.inl (Nat.lt_succ_self _))
        · simp only [h, if_false]
          have h' := fun hc => h (Ballot.blt_iff.mpr hc)
          refine Ballot.ble_iff.mpr ?_
          by_cases hnum : b.num = n
          · exact Or.inr ⟨hnum, by
              rcases Nat.lt_or_ge me.val b.proposerId.val with hp | hp
              · exact absurd (Or.inr ⟨hnum.symm, hp⟩) h'
              · exact hp⟩
          · refine Or.inl ?_
            rcases Nat.lt_trichotomy b.num n with hlt | heq | hgt
            · exact hlt
            · exact absurd heq hnum
            · exact absurd (Or.inl hgt) h'
    { own := hown
      hasLargest_true := hgl })

end HydroLean.Programs.Paxos
