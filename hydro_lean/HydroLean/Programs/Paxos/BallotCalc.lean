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

/-- Ballot-number update (paxos.rs:365–379): if the received max ballot beats
`Ballot { num: ballot_num, proposer_id: me }`, jump to `received.num + 1`. -/
def pBallotCalc (me : Fin nP) (ballotNum : Nat)
    (receivedMax : Option (Ballot nP)) : Nat :=
  match receivedMax with
  | some rm => if (Ballot.mk ballotNum me).blt rm then rm.num + 1 else ballotNum
  | none => ballotNum

/-- `p_ballot` (paxos.rs:381–384). -/
def pBallot (me : Fin nP) (ballotNum : Nat) (receivedMax : Option (Ballot nP)) :
    Ballot nP :=
  ⟨pBallotCalc me ballotNum receivedMax, me⟩

/-- `p_has_largest_ballot` (paxos.rs:386–390):
`received_max_ballot <= Some(cur_ballot)`. -/
def pHasLargestBallot (receivedMax : Option (Ballot nP)) (cur : Ballot nP) :
    Bool :=
  Ballot.optLe receivedMax cur

/-- The ballot number never decreases across ticks. -/
theorem pBallotCalc_mono (me : Fin nP) (n : Nat) (rm : Option (Ballot nP)) :
    n ≤ pBallotCalc me n rm := by
  unfold pBallotCalc
  cases rm with
  | none => exact Nat.le_refl n
  | some b =>
    by_cases h : (Ballot.mk n me).blt b
    · simp only [h, if_true]
      have h' := Ballot.blt_iff.mp h
      simp only [Ballot.num_mk, Ballot.proposerId_mk] at h'
      rcases h' with h' | ⟨h', _⟩ <;> omega
    · simp [h]

/-- After the update, our ballot is never behind the view: the jump always
overtakes the received max (`p_has_largest_ballot` holds of the *post-update*
ballot whenever it was computed from the same view). -/
theorem pBallot_overtakes (me : Fin nP) (n : Nat) (rm : Option (Ballot nP)) :
    pHasLargestBallot rm (pBallot me n rm) = true := by
  unfold pHasLargestBallot pBallot pBallotCalc Ballot.optLe
  cases rm with
  | none => rfl
  | some b =>
    by_cases h : (Ballot.mk n me).blt b
    · -- jumped to b.num + 1 > b.num: b ≤ (b.num+1, me) strictly by num
      simp only [h, if_true]
      exact Ballot.ble_iff.mpr (Or.inl (Nat.lt_succ_self _))
    · -- did not jump: ¬((n, me) < b) means b ≤ (n, me) (totality of lex)
      simp only [h, if_false]
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

open HydroLean.Hydro in
/-- paxos.rs:348–412 `p_ballot_calc`, as the typed dataflow — one `let` per
Rust binding; wiring is composition. The type carries both guarantees at
once: `.f` is the transcribed function, `.mono` the input-growth face
("realized view ticks are final"), and the ballot leg's `MonoSing` output
is its Rust `Monotonic` bound — the `use::state` fold carries its
`monotone =` justification (`pBallotCalc_mono`), so consumers read
cross-tick growth off the signature (`.ascending`), never rederive it. -/
def p_ballot_calcM (me : Fin nP) :
    TSing (Option (Ballot nP))
      →ₘ MonoSing (ballotNumVO (nP := nP)) × TSing Bool :=
  -- let mut p_ballot_num = use::state(|l| l.singleton(q!(0)));
  -- p_ballot_num = p_received_max_ballot.zip(p_ballot_num).map(…jump…)
  --   [monotonic = pBallotCalc_mono]
  let p_ballot_num := fold_monotonicM ValueOrder.nat
    (fun ballot_num received_max_ballot =>
      pBallotCalc me ballot_num received_max_ballot)
    0 (fun s x => pBallotCalc_mono me s x)
  -- let p_ballot = p_ballot_num.map(|num| Ballot { num, CLUSTER_SELF_ID })
  --   [order-preserving]
  let p_ballot := p_ballot_num.mapWire (fun num => Ballot.mk num me)
    (fun h => h)
  -- let p_has_largest_ballot = received.zip(p_ballot).map(rm <= Some(cur))
  let p_has_largest_ballot := (MonoMap.id.zip p_ballot.vals).map
    (fun rc => pHasLargestBallot rc.1 rc.2)
  MonoMap.pair p_ballot p_has_largest_ballot

open HydroLean.Hydro in
/-- The transcription's function face (the single source is the typed
dataflow `p_ballot_calcM`; this is its `.f`). -/
abbrev p_ballot_calc (me : Fin nP)
    (p_received_max_ballot : TSing (Option (Ballot nP))) :
    MonoSing (ballotNumVO (nP := nP)) × TSing Bool :=
  (p_ballot_calcM me).f p_received_max_ballot

/-! ### Verified face -/

open HydroLean.Hydro

/-- Ballots are owned by construction. -/
theorem p_ballot_calc_own (me : Fin nP)
    (v : TSing (Option (Ballot nP))) :
    ∀ b ∈ (p_ballot_calc me v).1.vals, (b : Ballot nP).proposerId = me := by
  intro b hb
  obtain ⟨num, -, rfl⟩ := List.mem_map.mp hb
  rfl

@[simp] theorem p_ballot_calc_length (me : Fin nP)
    (v : TSing (Option (Ballot nP))) :
    (p_ballot_calc me v).1.vals.length = v.length := by
  show ((scanSt _ 0 v).map _).length = _
  rw [List.length_map, scanSt_length]

/-- The ballot number visible at tick `t` (model equation). -/
theorem p_ballot_calc_getElem (me : Fin nP)
    (v : TSing (Option (Ballot nP))) (t : Nat)
    (ht : t < (p_ballot_calc me v).1.vals.length) :
    (p_ballot_calc me v).1.vals[t]'ht
      = Ballot.mk ((v.take (t + 1)).foldl (pBallotCalc me) 0) me := by
  have ht' : t < ((scanSt
      (fun ballot_num received_max_ballot =>
        pBallotCalc me ballot_num received_max_ballot) 0 v).map
      (fun num => Ballot.mk num me)).length := ht
  show ((scanSt _ 0 v).map (fun num => Ballot.mk num me))[t]'ht' = _
  rw [List.getElem_map, scanSt_getElem]

/-- `p_has_largest_ballot` is identically `true` on realized ticks (the
zip pairs each view with the ballot computed *from that view*, and the jump
always overtakes — `pBallot_overtakes`). Rust-visible spec face
(paxos.rs:386–390's wire); consumed by the `leader_ballot_stable`
derivation (`saGl_true`: a non-leader tick can only be bucket-masked,
never has-largest-false). -/
theorem p_ballot_calc_hasLargest (me : Fin nP)
    (v : TSing (Option (Ballot nP))) :
    ∀ g ∈ (p_ballot_calc me v).2, g = true := by
  intro g hg
  obtain ⟨t, ht, hgt⟩ := List.mem_iff_getElem.mp hg
  subst hgt
  have hlen : (p_ballot_calc me v).2.length ≤ v.length := by
    show ((TSing.zip _ _).map _).length ≤ _
    simp only [TSing.zip, List.length_map, List.length_zip]
    exact Nat.min_le_left _ _
  have htv : t < v.length := Nat.lt_of_lt_of_le ht hlen
  have htb : t < (p_ballot_calc me v).1.vals.length := by
    rw [p_ballot_calc_length]
    exact htv
  have hzip : t < (TSing.zip v (p_ballot_calc me v).1.vals).length := by
    simp only [TSing.zip, List.length_zip, p_ballot_calc_length]
    omega
  have hmap : t < ((TSing.zip v (p_ballot_calc me v).1.vals).map
      (fun rc => pHasLargestBallot rc.1 rc.2)).length := ht
  show ((TSing.zip v (p_ballot_calc me v).1.vals).map
    (fun rc => pHasLargestBallot rc.1 rc.2))[t]'hmap = true
  rw [List.getElem_map]
  have hpair : (TSing.zip v (p_ballot_calc me v).1.vals)[t]'hzip
      = (v[t]'htv, (p_ballot_calc me v).1.vals[t]'htb) := by
    simp only [TSing.zip]
    exact List.getElem_zip ..
  rw [hpair]
  rw [p_ballot_calc_getElem me v t htb]
  have hfold : (v.take (t + 1)).foldl (pBallotCalc me) 0
      = pBallotCalc me ((v.take t).foldl (pBallotCalc me) 0) (v[t]'htv) := by
    rw [List.take_succ, List.foldl_append]
    rw [List.getElem?_eq_getElem htv]
    rfl
  rw [hfold]
  exact pBallot_overtakes me _ _

end HydroLean.Programs.Paxos
