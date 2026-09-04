import HydroV2.MonoRel
import HydroV2.Paxos.Types
import Mathlib.Data.Multiset.Filter

/-!
# `p_p1b` · the pure quorum layer (quorum.rs helpers) and its lemmas

The keyed `fold_early_stop` / `get_max_key` closures `p_p1b` folds with
(mirroring paxos.rs:548–585 and quorum.rs), the realized view/flag
vocabulary, what `p_p1b` requires of its caller (`PP1bRequires`), and
the full lemma stack — bucket freeze/pin/count, `get_max_key`
domination, view accumulation, and the **fabricated-reign regress**
(`pP1b_no_false_full` / `pP1b_ballot_stable`, FINDINGS D21). The
program text and its colocated contract live in `PP1b.lean`.
-/

namespace HydroV2

variable {P : Type} [DecidableEq P]

/-- One insertion of the keyed `fold_early_stop` (paxos.rs:553–559):
push into the ballot's bucket until `quorum_size` logs are collected,
then stop. -/
def p1bLogsInsert {nP : Nat} (quorumSize : Nat)
    (logs : List (Ballot nP × List (ALog P nP)))
    (b : Ballot nP) (v : ALog P nP) :
    List (Ballot nP × List (ALog P nP)) :=
  match logs with
  | [] => [(b, [v])]
  | (b', vs) :: rest =>
    if b' = b then
      if vs.length < quorumSize then (b', vs ++ [v]) :: rest
      else (b', vs) :: rest
    else (b', vs) :: p1bLogsInsert quorumSize rest b v

/-- `get_max_key` over ballots holding a full quorum (paxos.rs:560). -/
def p1bMaxQuorumBallot {nP : Nat} (quorumSize : Nat)
    (logs : List (Ballot nP × List (ALog P nP))) :
    Option (Ballot nP × List (ALog P nP)) :=
  logs.foldl
    (fun acc e =>
      if quorumSize ≤ e.2.length then
        match acc with
        | none => some e
        | some a => if a.1.blt e.1 then some e else some a
      else acc)
    none

/-- `.zip(p_ballot).filter_map(quorum_ballot == my_ballot)`
(paxos.rs:573–581). -/
def pP1bQuorum {nP : Nat} (quorumSize : Nat)
    (view : List (Ballot nP × List (ALog P nP))) (myBallot : Ballot nP) :
    Option (List (ALog P nP)) :=
  match p1bMaxQuorumBallot quorumSize view with
  | some (qb, qlogs) => if qb = myBallot then some qlogs else none
  | none => none

/-- The `Ok`-response projection (`collect_quorum_with_response`'s
success leg). -/
def p1bOkPair {nP : Nat} (m : P1b P nP) :
    Option (Ballot nP × ALog P nP) :=
  match m.res with
  | .ok pl => some (m.ballot, pl)
  | .error _ => none



/-- The `fold_early_stop` keyed state as a pure function of the
selected consumption order (contract vocabulary: the body folds
`p1bLogsInsert` element-at-a-time through `H.fold`; this cumulative
form exists only for the faces below). -/
def foldEarlyStopBallots {nP : Nat} (quorumSize : Nat)
    (quorumOuts : List (Ballot nP × ALog P nP)) :
    List (Ballot nP × List (ALog P nP)) :=
  quorumOuts.foldl (fun acc bv => p1bLogsInsert quorumSize acc bv.1 bv.2) []


/-- The realized bucket views one proposer's snapshot cuts expose. -/
def pP1bViews (nP : Nat) (quorumSize : Nat)
    (pool : Multiset (P1b P nP))
    (dOrd : List (Ballot nP × ALog P nP)) (dSnap : List Nat) :
    Trace (List (Ballot nP × List (ALog P nP))) :=
  (prefixCuts (selectOrder (Multiset.filterMap p1bOkPair pool) dOrd)
      0 dSnap).map
    (fun sel => foldEarlyStopBallots quorumSize sel)


/-- The realized `p_is_leader` trace. -/
def pP1bFlags (nP : Nat) (quorumSize : Nat) (pool : Multiset (P1b P nP))
    (pb : Trace (Ballot nP)) (phl : Trace Bool)
    (dOrd : List (Ballot nP × ALog P nP)) (dSnap : List Nat) :
    Trace Bool :=
  (Trace.zip ((Trace.zip (pP1bViews nP quorumSize pool dOrd dSnap)
      pb).map (fun vb => pP1bQuorum quorumSize vb.1 vb.2)) phl).map
    (fun ql => ql.1.isSome && ql.2)


structure PP1bRequires (nP : Nat) (quorumSize : Nat)
    (pool : Multiset (P1b P nP)) (pb : Trace (Ballot nP))
    (phl : Trace Bool) (dOrd : List (Ballot nP × ALog P nP))
    (dSnap : List Nat) (me : Fin nP) : Prop where
  /-- `p_has_largest_ballot` is identically `true` on realized ticks —
  `p_ballot_calc`'s overtake guarantee (`PBCEnsures.hasLargest_true`). -/
  has_largest : ∀ g ∈ phl, g = true
  /-- Every realized ballot is the member's own
  (`PBCEnsures.own`). -/
  ballot_own : ∀ b ∈ pb, (b : Ballot nP).proposerId = me
  /-- Ballot numbers only ascend along the tick trace — the `Monotonic`
  wire type of `p_ballot`, projected. -/
  ballot_mono : ∀ {t t' : Nat} (h : t ≤ t') (ht' : t' < pb.length),
    (pb[t]'(Nat.lt_of_le_of_lt h ht')).num ≤ (pb[t']'ht').num
  /-- **Solicitation staging**: every `Ok` promise in the reply pool was
  solicited at a realized tick that carried its ballot and read a
  `false` leader flag — the election trigger only fires at followers
  (the `forward_ref` cycle's staging; the fabricated-reign regress's
  entry point). -/
  solicited_at_follower : ∀ m ∈ pool, (∃ v, (m : P1b P nP).res = .ok v) →
    ∃ (u : Nat)
      (hu : u < (pP1bFlags nP quorumSize pool pb phl dOrd dSnap).length)
      (hb : u < pb.length),
      pb[u]'hb = m.ballot
      ∧ (pP1bFlags nP quorumSize pool pb phl dOrd dSnap)[u]'hu = false



/-- `get_max_key` only surfaces full buckets (paxos.rs:560 after
`fold_early_stop`'s stop condition). -/
theorem p1bMaxQuorumBallot_holds_quorum {nP : Nat} (quorumSize : Nat)
    (logs : List (Ballot nP × List (ALog P nP)))
    {qb : Ballot nP} {qlogs : List (ALog P nP)}
    (h : p1bMaxQuorumBallot quorumSize logs = some (qb, qlogs)) :
    quorumSize ≤ qlogs.length := by
  unfold p1bMaxQuorumBallot at h
  suffices hgen : ∀ (l : List (Ballot nP × List (ALog P nP)))
      (acc : Option (Ballot nP × List (ALog P nP))),
      (∀ a, acc = some a → quorumSize ≤ a.2.length) →
      ∀ r, l.foldl (fun acc e =>
        if quorumSize ≤ e.2.length then
          match acc with
          | none => some e
          | some a => if a.1.blt e.1 then some e else some a
        else acc) acc = some r → quorumSize ≤ r.2.length by
    exact hgen logs none (fun a ha => nomatch ha) (qb, qlogs) h
  intro l
  induction l with
  | nil => exact fun acc hacc r h' => hacc r h'
  | cons e rest ih =>
    intro acc hacc r h'
    simp only [List.foldl_cons] at h'
    refine ih _ ?_ r h'
    intro a ha
    by_cases hq : quorumSize ≤ e.2.length
    · rw [if_pos hq] at ha
      cases hacc' : acc with
      | none => rw [hacc'] at ha; cases ha; exact hq
      | some a' =>
        rw [hacc'] at ha
        have ha' : (if a'.1.blt e.1 = true then some e else some a')
            = some a := ha
        by_cases hlt : a'.1.blt e.1
        · rw [if_pos hlt] at ha'; cases ha'; exact hq
        · rw [if_neg hlt] at ha'; cases ha'; exact hacc _ hacc'
    · rw [if_neg hq] at ha
      exact hacc _ ha

/-- `get_max_key`'s result is one of the buckets. -/
theorem p1bMaxQuorumBallot_mem_self {nP : Nat} {quorumSize : Nat}
    {logs : List (Ballot nP × List (ALog P nP))}
    {r : Ballot nP × List (ALog P nP)}
    (h : p1bMaxQuorumBallot quorumSize logs = some r) : r ∈ logs := by
  unfold p1bMaxQuorumBallot at h
  suffices hgen : ∀ (l : List (Ballot nP × List (ALog P nP)))
      (acc : Option (Ballot nP × List (ALog P nP))),
      (∀ a, acc = some a → a ∈ logs) → (∀ x ∈ l, x ∈ logs) →
      ∀ r, l.foldl (fun acc e =>
        if quorumSize ≤ e.2.length then
          match acc with
          | none => some e
          | some a => if a.1.blt e.1 then some e else some a
        else acc) acc = some r → r ∈ logs by
    exact hgen logs none (fun a ha => nomatch ha) (fun x hx => hx) r h
  intro l
  induction l with
  | nil =>
    intro acc hacc _ r hr
    rw [List.foldl_nil] at hr
    exact hacc r hr
  | cons x rest ih =>
    intro acc hacc hmem r hr
    rw [List.foldl_cons] at hr
    refine ih _ ?_ (fun y hy => hmem y (List.mem_cons_of_mem _ hy)) r hr
    intro a ha
    by_cases hq : quorumSize ≤ x.2.length
    · rw [if_pos hq] at ha
      cases hacc' : acc with
      | none =>
        rw [hacc'] at ha
        dsimp only at ha
        cases ha
        exact hmem x List.mem_cons_self
      | some a' =>
        rw [hacc'] at ha
        dsimp only at ha
        by_cases hlt : a'.1.blt x.1
        · rw [if_pos hlt] at ha
          cases ha
          exact hmem x List.mem_cons_self
        · rw [if_neg hlt] at ha
          cases ha
          exact hacc _ hacc'
    · rw [if_neg hq] at ha
      exact hacc _ ha

/-- Insertion source-tracking: every stored payload is an old one or the
inserted pair. -/
theorem p1bLogsInsert_src {nP : Nat} {quorumSize : Nat}
    {C : Ballot nP × ALog P nP → Prop}
    {logs : List (Ballot nP × List (ALog P nP))}
    {b : Ballot nP} {v : ALog P nP}
    (hold : ∀ e ∈ logs, ∀ w ∈ e.2, C (e.1, w)) (hnew : C (b, v)) :
    ∀ e ∈ p1bLogsInsert quorumSize logs b v, ∀ w ∈ e.2, C (e.1, w) := by
  induction logs with
  | nil =>
    intro e he w hw
    rcases List.mem_cons.mp he with rfl | he'
    · rcases List.mem_cons.mp hw with rfl | hw'
      · exact hnew
      · cases hw'
    · cases he'
  | cons p rest ih =>
    obtain ⟨b', vs⟩ := p
    intro e he w hw
    rw [show p1bLogsInsert quorumSize ((b', vs) :: rest) b v
        = if b' = b then
            if vs.length < quorumSize then (b', vs ++ [v]) :: rest
            else (b', vs) :: rest
          else (b', vs) :: p1bLogsInsert quorumSize rest b v from rfl]
      at he
    by_cases hb : b' = b
    · rw [if_pos hb] at he
      by_cases hlen : vs.length < quorumSize
      · rw [if_pos hlen] at he
        rcases List.mem_cons.mp he with rfl | he'
        · rcases List.mem_append.mp hw with hw' | hw'
          · exact hold (b', vs) List.mem_cons_self w hw'
          · rcases List.mem_cons.mp hw' with rfl | hw''
            · rw [hb]
              exact hnew
            · cases hw''
        · exact hold e (List.mem_cons_of_mem _ he') w hw
      · rw [if_neg hlen] at he
        exact hold e he w hw
    · rw [if_neg hb] at he
      rcases List.mem_cons.mp he with rfl | he'
      · exact hold (b', vs) List.mem_cons_self w hw
      · exact ih (fun e' he' w' hw' =>
          hold e' (List.mem_cons_of_mem _ he') w' hw') e he' w hw

/-- Bucket source-tracking through the whole fold: every stored payload
was one of the consumed quorum outputs. -/
theorem foldEarlyStop_src {nP : Nat} (quorumSize : Nat)
    (outs : List (Ballot nP × ALog P nP)) :
    ∀ e ∈ foldEarlyStopBallots (P := P) quorumSize outs,
      ∀ w ∈ e.2, (e.1, w) ∈ outs := by
  suffices hgen : ∀ (l acc' : List _) (C : Ballot nP × ALog P nP → Prop),
      (∀ e ∈ acc', ∀ w ∈ (e :
          Ballot nP × List (ALog P nP)).2, C (e.1, w)) →
      (∀ o ∈ l, C o) →
      ∀ e ∈ l.foldl
        (fun acc bv => p1bLogsInsert quorumSize acc bv.1 bv.2) acc',
        ∀ w ∈ e.2, C (e.1, w) by
    exact hgen outs [] (· ∈ outs) (fun e he => nomatch he) (fun o ho => ho)
  intro l
  induction l with
  | nil => exact fun acc' C hacc _ e he w hw => hacc e he w hw
  | cons o rest ih =>
    intro acc' C hacc hl e he w hw
    rw [List.foldl_cons] at he
    exact ih _ C
      (p1bLogsInsert_src hacc (hl o List.mem_cons_self))
      (fun o' ho' => hl o' (List.mem_cons_of_mem _ ho'))
      e he w hw



private theorem eq_of_nodup_keys {α κ : Type _} {l : List α} {g : α → κ}
    (hl : (l.map g).Nodup) {x y : α} (hx : x ∈ l) (hy : y ∈ l)
    (hxy : g x = g y) : x = y := by
  induction l with
  | nil => cases hx
  | cons hd rest ih =>
    rw [List.map_cons, List.nodup_cons] at hl
    rcases List.mem_cons.mp hx with rfl | hxr
    · rcases List.mem_cons.mp hy with rfl | hyr
      · rfl
      · exact absurd (List.mem_map.mpr ⟨y, hyr, hxy.symm⟩) hl.1
    · rcases List.mem_cons.mp hy with rfl | hyr
      · exact absurd (List.mem_map.mpr ⟨x, hxr, hxy⟩) hl.1
      · exact ih hl.2 hxr hyr

/-- The fold, in `foldl`-over-pairs form. -/
theorem foldEarlyStopBallots_eq_foldl {nP : Nat} (quorumSize : Nat)
    (quorumOuts : List (Ballot nP × ALog P nP)) :
    foldEarlyStopBallots quorumSize quorumOuts
      = quorumOuts.foldl
          (fun acc e => p1bLogsInsert quorumSize acc e.1 e.2) [] := rfl

/-- The fold appends: extending the source continues the fold. -/
theorem foldEarlyStop_append {nP : Nat} (quorumSize : Nat)
    (pfx ext : List (Ballot nP × ALog P nP)) :
    foldEarlyStopBallots quorumSize (pfx ++ ext)
      = ext.foldl (fun acc e => p1bLogsInsert quorumSize acc e.1 e.2)
          (foldEarlyStopBallots quorumSize pfx) := by
  rw [foldEarlyStopBallots_eq_foldl, foldEarlyStopBallots_eq_foldl,
    List.foldl_append]

/-- Insertion never removes or reorders buckets: membership after an insert
is an old entry, the updated bucket (appended, if not full), or a fresh
singleton bucket. -/
theorem p1bLogsInsert_mem_cases {nP : Nat} {quorumSize : Nat}
    {logs : List (Ballot nP × List (ALog P nP))}
    {b : Ballot nP} {v : ALog P nP}
    {qb : Ballot nP} {vs' : List (ALog P nP)}
    (h : (qb, vs') ∈ p1bLogsInsert quorumSize logs b v) :
    (qb, vs') ∈ logs ∨
      (qb = b ∧
        ((∃ vs, (b, vs) ∈ logs ∧ vs.length < quorumSize ∧ vs' = vs ++ [v]) ∨
          vs' = [v])) := by
  induction logs with
  | nil =>
    have := List.mem_singleton.mp h
    cases this
    exact Or.inr ⟨rfl, Or.inr rfl⟩
  | cons hd rest ih =>
    obtain ⟨b', vs⟩ := hd
    simp only [p1bLogsInsert] at h
    by_cases hb : b' = b
    · rw [if_pos hb] at h
      by_cases hlen : vs.length < quorumSize
      · rw [if_pos hlen] at h
        rcases List.mem_cons.mp h with heq | hmem
        · cases heq
          exact Or.inr ⟨hb, Or.inl ⟨vs, hb ▸ List.mem_cons_self .., hlen, rfl⟩⟩
        · exact Or.inl (List.mem_cons_of_mem _ hmem)
      · rw [if_neg hlen] at h
        exact Or.inl h
    · rw [if_neg hb] at h
      rcases List.mem_cons.mp h with heq | hmem
      · exact Or.inl (heq ▸ List.mem_cons_self ..)
      · rcases ih hmem with hold | ⟨hqb, hcase⟩
        · exact Or.inl (List.mem_cons_of_mem _ hold)
        · refine Or.inr ⟨hqb, ?_⟩
          rcases hcase with ⟨vs₀, hvs₀, hlen₀, heq₀⟩ | hnew
          · exact Or.inl ⟨vs₀, List.mem_cons_of_mem _ hvs₀, hlen₀, heq₀⟩
          · exact Or.inr hnew

/-- Insertion preserves duplicate-free bucket keys. -/
theorem p1bLogsInsert_keys_nodup {nP : Nat} {quorumSize : Nat}
    {logs : List (Ballot nP × List (ALog P nP))}
    (hnd : (logs.map Prod.fst).Nodup) (b : Ballot nP) (v : ALog P nP) :
    ((p1bLogsInsert quorumSize logs b v).map Prod.fst).Nodup := by
  induction logs with
  | nil => exact List.pairwise_singleton _ _
  | cons hd rest ih =>
    obtain ⟨b', vs⟩ := hd
    rw [List.map_cons, List.nodup_cons] at hnd
    simp only [p1bLogsInsert]
    by_cases hb : b' = b
    · rw [if_pos hb]
      by_cases hlen : vs.length < quorumSize
      · rw [if_pos hlen, List.map_cons, List.nodup_cons]
        exact ⟨hnd.1, hnd.2⟩
      · rw [if_neg hlen, List.map_cons, List.nodup_cons]
        exact ⟨hnd.1, hnd.2⟩
    · rw [if_neg hb, List.map_cons, List.nodup_cons]
      refine ⟨fun hc => ?_, ih hnd.2⟩
      obtain ⟨⟨qb, vs'⟩, hmem, hkey⟩ := List.mem_map.mp hc
      have hkey' : qb = b' := hkey
      rcases p1bLogsInsert_mem_cases hmem with hold | ⟨hqb, -⟩
      · exact hnd.1 (List.mem_map.mpr ⟨_, hold, hkey⟩)
      · exact hb (hkey' ▸ hqb)

/-- The fold's bucket keys are duplicate-free. -/
theorem foldEarlyStop_keys_nodup {nP : Nat} (quorumSize : Nat)
    (quorumOuts : List (Ballot nP × ALog P nP)) :
    ((foldEarlyStopBallots quorumSize quorumOuts).map Prod.fst).Nodup := by
  rw [foldEarlyStopBallots_eq_foldl]
  have hgen : ∀ (l : List (Ballot nP × ALog P nP))
      (acc : List (Ballot nP × List (ALog P nP))),
      (acc.map Prod.fst).Nodup →
      ((l.foldl (fun acc e => p1bLogsInsert quorumSize acc e.1 e.2)
        acc).map Prod.fst).Nodup := by
    intro l
    induction l with
    | nil => exact fun acc h => h
    | cons x rest ih =>
      intro acc hacc
      rw [List.foldl_cons]
      exact ih _ (p1bLogsInsert_keys_nodup hacc x.1 x.2)
  exact hgen _ [] List.Pairwise.nil

/-- `fold_early_stop` never collects more than `quorum_size` logs per ballot
(the early stop; paxos.rs:557), for `1 ≤ quorum_size`. With
sender-deduplicated inputs (the guarded variant's contract) this makes a full
vector a genuine quorum. -/
theorem p1bLogsInsert_length_le {nP : Nat} (quorumSize : Nat) (hq1 : 1 ≤ quorumSize)
    (logs : List (Ballot nP × List (ALog P nP)))
    (b : Ballot nP) (v : ALog P nP)
    (h : ∀ e ∈ logs, e.2.length ≤ quorumSize) :
    ∀ e ∈ p1bLogsInsert quorumSize logs b v, e.2.length ≤ quorumSize := by
  induction logs with
  | nil =>
    intro e he
    rcases List.mem_singleton.mp he with rfl
    simpa using hq1
  | cons hd rest ih =>
    intro e he
    obtain ⟨b', vs⟩ := hd
    simp only [p1bLogsInsert] at he
    by_cases hb : b' = b
    · rw [if_pos hb] at he
      by_cases hlen : vs.length < quorumSize
      · rw [if_pos hlen] at he
        rcases List.mem_cons.mp he with rfl | hmem
        · simpa using Nat.succ_le_of_lt hlen
        · exact h e (List.mem_cons_of_mem _ hmem)
      · rw [if_neg hlen] at he
        exact h e he
    · rw [if_neg hb] at he
      rcases List.mem_cons.mp he with rfl | hmem
      · exact h _ List.mem_cons_self
      · exact ih (fun e' he' => h e' (List.mem_cons_of_mem _ he')) e hmem

/-- A full bucket survives one insertion untouched. -/
theorem p1bLogsInsert_full_stable {nP : Nat} {quorumSize : Nat}
    {logs : List (Ballot nP × List (ALog P nP))}
    {qb : Ballot nP} {vs : List (ALog P nP)}
    (hmem : (qb, vs) ∈ logs) (hfull : vs.length = quorumSize)
    (b : Ballot nP) (v : ALog P nP) :
    (qb, vs) ∈ p1bLogsInsert quorumSize logs b v := by
  induction logs with
  | nil => cases hmem
  | cons hd rest ih =>
    obtain ⟨b', vs'⟩ := hd
    simp only [p1bLogsInsert]
    by_cases hb : b' = b
    · rw [if_pos hb]
      by_cases hlen : vs'.length < quorumSize
      · rw [if_pos hlen]
        rcases List.mem_cons.mp hmem with heq | hmemr
        · -- the head bucket would be full, contradicting `hlen`
          cases heq
          omega
        · exact List.mem_cons_of_mem _ hmemr
      · rw [if_neg hlen]
        exact hmem
    · rw [if_neg hb]
      rcases List.mem_cons.mp hmem with heq | hmemr
      · exact heq ▸ List.mem_cons_self ..
      · exact List.mem_cons_of_mem _ (ih hmemr)

/-- **Full buckets are stable**: once a ballot's bucket is full, it is the
same at every later cut of the source stream — the elected quorum view is a
single value per `(proposer, ballot)`. -/
theorem foldEarlyStop_full_stable {nP : Nat} {quorumSize : Nat}
    {pfx : List (Ballot nP × ALog P nP)}
    {qb : Ballot nP} {vs : List (ALog P nP)}
    (hmem : (qb, vs) ∈ foldEarlyStopBallots quorumSize pfx)
    (hfull : vs.length = quorumSize)
    (ext : List (Ballot nP × ALog P nP)) :
    (qb, vs) ∈ foldEarlyStopBallots quorumSize (pfx ++ ext) := by
  rw [foldEarlyStop_append]
  have hgen : ∀ (l : List (Ballot nP × ALog P nP))
      (acc : List (Ballot nP × List (ALog P nP))),
      (qb, vs) ∈ acc →
      (qb, vs) ∈ l.foldl
        (fun acc e => p1bLogsInsert quorumSize acc e.1 e.2) acc := by
    intro l
    induction l with
    | nil => exact fun acc h => h
    | cons x rest ih =>
      intro acc hacc
      rw [List.foldl_cons]
      exact ih _ (p1bLogsInsert_full_stable hacc hfull x.1 x.2)
  exact hgen ext _ hmem

/-- All buckets hold at most `quorum_size` logs (`1 ≤ quorum_size`). -/
theorem foldEarlyStop_length_le {nP : Nat} {quorumSize : Nat} (hq1 : 1 ≤ quorumSize)
    (pfx : List (Ballot nP × ALog P nP)) :
    ∀ e ∈ foldEarlyStopBallots quorumSize pfx, e.2.length ≤ quorumSize := by
  rw [foldEarlyStopBallots_eq_foldl]
  have hgen : ∀ (l : List (Ballot nP × ALog P nP))
      (acc : List (Ballot nP × List (ALog P nP))),
      (∀ e ∈ acc, e.2.length ≤ quorumSize) →
      ∀ e ∈ l.foldl (fun acc e => p1bLogsInsert quorumSize acc e.1 e.2) acc,
        e.2.length ≤ quorumSize := by
    intro l
    induction l with
    | nil => exact fun acc h => h
    | cons x rest ih =>
      intro acc hacc
      rw [List.foldl_cons]
      exact ih _ (p1bLogsInsert_length_le quorumSize hq1 acc x.1 x.2 hacc)
  exact hgen _ [] (fun e he => nomatch he)

/-- **Bucket counting**: a bucket's per-value multiplicities are covered by
the source stream's `(ballot, value)` multiplicities — collected quorum
entries are genuine emissions, *with multiplicity*. -/
theorem foldEarlyStop_count {nP : Nat} [DecidableEq P] {quorumSize : Nat}
    {pfx : List (Ballot nP × ALog P nP)}
    {qb : Ballot nP} {vs : List (ALog P nP)}
    (hmem : (qb, vs) ∈ foldEarlyStopBallots quorumSize pfx)
    (v : ALog P nP) :
    vs.countP (fun w => decide (w = v))
      ≤ pfx.countP (fun e => decide (e.1 = qb) && decide (e.2 = v)) := by
  rw [foldEarlyStopBallots_eq_foldl] at hmem
  -- fold invariant with a per-key bound function
  have hgen : ∀ (l : List (Ballot nP × ALog P nP))
      (acc : List (Ballot nP × List (ALog P nP)))
      (C : Ballot nP → Nat),
      (∀ e ∈ acc, e.2.countP (fun w => decide (w = v)) ≤ C e.1) →
      ∀ e ∈ l.foldl (fun acc e => p1bLogsInsert quorumSize acc e.1 e.2) acc,
        e.2.countP (fun w => decide (w = v))
          ≤ C e.1 + l.countP (fun x => decide (x.1 = e.1) && decide (x.2 = v)) := by
    intro l
    induction l with
    | nil =>
      intro acc C hacc e he
      have := hacc e he
      rw [List.countP_nil]
      omega
    | cons x rest ih =>
      intro acc C hacc e he
      rw [List.foldl_cons] at he
      -- absorb `x` into the per-key bound
      have hstep := ih (p1bLogsInsert quorumSize acc x.1 x.2)
        (fun k => C k + (if x.1 = k ∧ x.2 = v then 1 else 0))
        (by
          intro e' he'
          rcases p1bLogsInsert_mem_cases he' with hold | ⟨hqb', hcase⟩
          · have := hacc e' hold
            have hnn : (0 : Nat) ≤ (if x.1 = e'.1 ∧ x.2 = v then 1 else 0) :=
              Nat.zero_le _
            omega
          · rcases hcase with ⟨vs₀, hvs₀, -, heq₀⟩ | hnew
            · have hbase : vs₀.countP (fun w => decide (w = v)) ≤ C x.1 :=
                hacc (x.1, vs₀) (hqb' ▸ hvs₀)
              have hkeq : e'.1 = x.1 := hqb'
              rw [show e'.2 = vs₀ ++ [x.2] from heq₀, List.countP_append,
                List.countP_cons, List.countP_nil, hkeq]
              by_cases hv : x.2 = v
              · rw [if_pos (by simpa using hv),
                  if_pos (show x.1 = x.1 ∧ x.2 = v from ⟨rfl, hv⟩)]
                omega
              · rw [if_neg (by simpa using hv), if_neg (by
                  rintro ⟨-, hc⟩
                  exact hv hc)]
                omega
            · have hkeq : e'.1 = x.1 := hqb'
              rw [show e'.2 = [x.2] from hnew, List.countP_cons,
                List.countP_nil, hkeq]
              by_cases hv : x.2 = v
              · rw [if_pos (by simpa using hv),
                  if_pos (show x.1 = x.1 ∧ x.2 = v from ⟨rfl, hv⟩)]
                omega
              · rw [if_neg (by simpa using hv), if_neg (by
                  rintro ⟨-, hc⟩
                  exact hv hc)]
                omega)
        e he
      rw [List.countP_cons]
      by_cases hx : (decide (x.1 = e.1) && decide (x.2 = v)) = true
      · rw [if_pos hx]
        rw [Bool.and_eq_true, decide_eq_true_iff, decide_eq_true_iff] at hx
        have hone : (if x.1 = e.1 ∧ x.2 = v then 1 else 0) = 1 := if_pos hx
        omega
      · rw [if_neg hx]
        have hzero : (if x.1 = e.1 ∧ x.2 = v then 1 else 0) = 0 := by
          refine if_neg ?_
          rintro ⟨h1, h2⟩
          exact hx (by
            rw [Bool.and_eq_true, decide_eq_true_iff, decide_eq_true_iff]
            exact ⟨h1, h2⟩)
        omega
  have := hgen pfx [] (fun _ => 0) (fun e he => nomatch he) (qb, vs) hmem
  simpa using this

/-- Two buckets of one ballot are the same bucket (bucket keys are
duplicate-free). -/
theorem foldEarlyStop_bucket_unique {nP : Nat} {quorumSize : Nat}
    {pfx : List (Ballot nP × ALog P nP)} {qb : Ballot nP}
    {vs vs' : List (ALog P nP)}
    (h : (qb, vs) ∈ foldEarlyStopBallots quorumSize pfx)
    (h' : (qb, vs') ∈ foldEarlyStopBallots quorumSize pfx) : vs = vs' :=
  congrArg Prod.snd (eq_of_nodup_keys (g := Prod.fst)
    (foldEarlyStop_keys_nodup quorumSize pfx) h h' rfl)

/-- **Frozen-bucket pinning**: a full bucket equals the same ballot's bucket
at any later cut — the "elected quorum view" is one value per ballot across
the whole run. -/
theorem foldEarlyStop_full_pin {nP : Nat} {quorumSize : Nat} (hq1 : 1 ≤ quorumSize)
    {pfx ext : List (Ballot nP × ALog P nP)} {qb : Ballot nP}
    {vs vs' : List (ALog P nP)}
    (hmem : (qb, vs) ∈ foldEarlyStopBallots quorumSize pfx)
    (hfull : quorumSize ≤ vs.length)
    (hmem' : (qb, vs') ∈ foldEarlyStopBallots quorumSize (pfx ++ ext)) :
    vs = vs' := by
  have hlen : vs.length = quorumSize :=
    Nat.le_antisymm (foldEarlyStop_length_le hq1 pfx _ hmem) hfull
  exact foldEarlyStop_bucket_unique (foldEarlyStop_full_stable hmem hlen ext)
    hmem'

/-! ## The contract, over the `Values` denotation -/

/-! ## `get_max_key` dominates full buckets -/

/-- A full bucket forces `get_max_key` to answer, at a key at least the
bucket's. -/
theorem p1bMaxQuorumBallot_ge_full {nP : Nat} (quorumSize : Nat)
    {logs : List (Ballot nP × List (ALog P nP))}
    {b : Ballot nP} {vs : List (ALog P nP)}
    (hmem : (b, vs) ∈ logs) (hfull : quorumSize ≤ vs.length) :
    ∃ r, p1bMaxQuorumBallot quorumSize logs = some r
      ∧ b.ble r.1 = true := by
  unfold p1bMaxQuorumBallot
  suffices hgen : ∀ (l : List (Ballot nP × List (ALog P nP)))
      (acc : Option (Ballot nP × List (ALog P nP))),
      ((∃ a, acc = some a ∧ b.ble a.1 = true) ∨ (b, vs) ∈ l) →
      ∃ r, l.foldl (fun acc e =>
        if quorumSize ≤ e.2.length then
          match acc with
          | none => some e
          | some a => if a.1.blt e.1 then some e else some a
        else acc) acc = some r ∧ b.ble r.1 = true by
    exact hgen logs none (Or.inr hmem)
  intro l
  induction l with
  | nil =>
    rintro acc (⟨a, rfl, hba⟩ | hmem')
    · exact ⟨a, rfl, hba⟩
    · cases hmem'
  | cons e rest ih =>
    rintro acc (⟨a, rfl, hba⟩ | hmem')
    · rw [List.foldl_cons]
      refine ih _ (Or.inl ?_)
      by_cases hq : quorumSize ≤ e.2.length
      · rw [if_pos hq]
        dsimp only
        by_cases hlt : a.1.blt e.1
        · rw [if_pos hlt]
          exact ⟨e, rfl, Ballot.ble_trans hba (Ballot.ble_of_blt hlt)⟩
        · rw [if_neg hlt]
          exact ⟨a, rfl, hba⟩
      · rw [if_neg hq]
        exact ⟨a, rfl, hba⟩
    · rw [List.foldl_cons]
      rcases List.mem_cons.mp hmem' with rfl | hrest
      · refine ih _ (Or.inl ?_)
        rw [if_pos hfull]
        cases acc with
        | none => exact ⟨(b, vs), rfl, Ballot.ble_refl b⟩
        | some a =>
          dsimp only
          by_cases hlt : a.1.blt b
          · rw [if_pos hlt]
            exact ⟨(b, vs), rfl, Ballot.ble_refl b⟩
          · rw [if_neg hlt]
            exact ⟨a, rfl, Ballot.ble_of_not_blt (by simpa using hlt)⟩
      · exact ih _ (Or.inr hrest)

/-! ## Realized views accumulate -/

/-- A full bucket carries to every later realized view (frozen). -/
theorem pP1bViews_full_carry {nP : Nat} (quorumSize : Nat)
    (pool : Multiset (P1b P nP))
    (dOrd : List (Ballot nP × ALog P nP)) (dSnap : List Nat)
    {t t' : Nat} (h : t ≤ t')
    (ht' : t' < (pP1bViews nP quorumSize pool dOrd dSnap).length)
    {qb : Ballot nP} {vs : List (ALog P nP)}
    (hmem : (qb, vs) ∈ (pP1bViews nP quorumSize pool dOrd
      dSnap)[t]'(Nat.lt_of_le_of_lt h ht'))
    (hfull : vs.length = quorumSize) :
    (qb, vs) ∈ (pP1bViews nP quorumSize pool dOrd dSnap)[t']'ht' := by
  unfold pP1bViews at *
  have ht0 : t < (prefixCuts (selectOrder
      (Multiset.filterMap p1bOkPair pool) dOrd) 0 dSnap).length := by
    have := Nat.lt_of_le_of_lt h ht'
    rwa [List.length_map] at this
  have ht0' : t' < (prefixCuts (selectOrder
      (Multiset.filterMap p1bOkPair pool) dOrd) 0 dSnap).length := by
    rwa [List.length_map] at ht'
  rw [List.getElem_map] at hmem ⊢
  obtain ⟨ext, hext⟩ := prefixCuts_getElem_prefix (pool := selectOrder
    (Multiset.filterMap p1bOkPair pool) dOrd) (d := dSnap) h ht0'
  rw [← hext]
  exact foldEarlyStop_full_stable hmem hfull ext

/-- Every payload of a realized bucket is an `Ok` reply in the pool at
the bucket's ballot. -/
theorem pP1bViews_entry_ok {nP : Nat} (quorumSize : Nat)
    (pool : Multiset (P1b P nP))
    (dOrd : List (Ballot nP × ALog P nP)) (dSnap : List Nat)
    {t : Nat} (ht : t < (pP1bViews nP quorumSize pool dOrd dSnap).length)
    {qb : Ballot nP} {vs : List (ALog P nP)}
    (hmem : (qb, vs) ∈ (pP1bViews nP quorumSize pool dOrd dSnap)[t]'ht) :
    ∀ v ∈ vs, ∃ m ∈ pool, m.ballot = qb ∧ m.res = .ok v := by
  intro v hv
  unfold pP1bViews at ht hmem
  have ht0 : t < (prefixCuts (selectOrder
      (Multiset.filterMap p1bOkPair pool) dOrd) 0 dSnap).length := by
    rwa [List.length_map] at ht
  rw [List.getElem_map] at hmem
  have hpair := foldEarlyStop_src quorumSize _ (qb, vs) hmem v hv
  obtain ⟨k, hk⟩ := prefixCuts_mem_take (List.getElem_mem ht0)
  rw [hk] at hpair
  have hsel : ((qb, v) : Ballot nP × ALog P nP)
      ∈ selectOrder (Multiset.filterMap p1bOkPair pool) dOrd :=
    (List.take_sublist ..).subset hpair
  obtain ⟨m, hm, hok⟩ :=
    (Multiset.mem_filterMap _ _).mp (selectOrder_mem _ hsel)
  refine ⟨m, hm, ?_, ?_⟩
  · unfold p1bOkPair at hok
    cases hres : m.res with
    | ok pl =>
      rw [hres] at hok
      injection hok with hok'
      exact congrArg Prod.fst hok'
    | error e =>
      rw [hres] at hok
      cases hok
  · unfold p1bOkPair at hok
    cases hres : m.res with
    | ok pl =>
      rw [hres] at hok
      injection hok with hok'
      have hpv : pl = v := congrArg Prod.snd hok'
      rw [hpv]
    | error e =>
      rw [hres] at hok
      cases hok

/-! ## The flag trace, pure (the contract vocabulary for the election
feedback: `p_p1b`'s `Values` flag output IS this trace) -/

/-- Open one realized flag tick into its components. -/
theorem pP1bFlags_getElem {nP : Nat} (quorumSize : Nat)
    (pool : Multiset (P1b P nP)) (pb : Trace (Ballot nP))
    (phl : Trace Bool) (dOrd : List (Ballot nP × ALog P nP))
    (dSnap : List Nat) {u : Nat}
    (hu : u < (pP1bFlags nP quorumSize pool pb phl dOrd dSnap).length) :
    ∃ (hv : u < (pP1bViews nP quorumSize pool dOrd dSnap).length)
      (hb : u < pb.length) (hl : u < phl.length),
      (pP1bFlags nP quorumSize pool pb phl dOrd dSnap)[u]'hu
        = ((pP1bQuorum quorumSize
            ((pP1bViews nP quorumSize pool dOrd dSnap)[u]'hv)
            (pb[u]'hb)).isSome && phl[u]'hl) := by
  have hlen := hu
  unfold pP1bFlags at hlen
  simp only [Trace.zip, List.length_map, List.length_zip] at hlen
  have hv : u < (pP1bViews nP quorumSize pool dOrd dSnap).length := by
    omega
  have hb : u < pb.length := by omega
  have hl : u < phl.length := by omega
  refine ⟨hv, hb, hl, ?_⟩
  unfold pP1bFlags
  simp only [Trace.zip]
  rw [List.getElem_map, List.getElem_zip, List.getElem_map,
    List.getElem_zip]

/-- All realized buckets hold at most `quorum_size` logs. -/
theorem pP1bViews_length_le {nP : Nat} (quorumSize : Nat)
    (hq1 : 1 ≤ quorumSize) (pool : Multiset (P1b P nP))
    (dOrd : List (Ballot nP × ALog P nP)) (dSnap : List Nat)
    {t : Nat} (ht : t < (pP1bViews nP quorumSize pool dOrd dSnap).length)
    {e : Ballot nP × List (ALog P nP)}
    (hmem : e ∈ (pP1bViews nP quorumSize pool dOrd dSnap)[t]'ht) :
    e.2.length ≤ quorumSize := by
  unfold pP1bViews at ht hmem
  rw [List.getElem_map] at hmem
  exact foldEarlyStop_length_le hq1 _ e hmem

private theorem count_ofList {α : Type _} [DecidableEq α] (x : α)
    (l : List α) :
    Multiset.count x (Multiset.ofList l)
      = l.countP (fun w => decide (w = x)) := by
  induction l with
  | nil => rfl
  | cons a l ih =>
    rw [show Multiset.ofList (a :: l) = a ::ₘ (Multiset.ofList l) from rfl,
      Multiset.count_cons, List.countP_cons, ih]
    by_cases h : a = x
    · simp [h]
    · simp [h]
      exact fun hc => h hc.symm

/-- **Bucket multiplicity embedding**: a realized bucket's `(ballot,
payload)` pairs form a sub-multiset of the pool's `Ok` projection —
collected quorum entries are genuine promises, *with multiplicity* (the
distinct-provider extraction composes this with the per-sender caps). -/
theorem pP1bViews_bucket_sub_oks {nP : Nat} (quorumSize : Nat)
    (pool : Multiset (P1b P nP))
    (dOrd : List (Ballot nP × ALog P nP)) (dSnap : List Nat)
    {t : Nat} (ht : t < (pP1bViews nP quorumSize pool dOrd dSnap).length)
    {qb : Ballot nP} {vs : List (ALog P nP)}
    (hmem : (qb, vs) ∈ (pP1bViews nP quorumSize pool dOrd dSnap)[t]'ht) :
    (Multiset.ofList (vs.map (fun v => ((qb, v) : Ballot nP × ALog P nP))))
      ≤ Multiset.filterMap p1bOkPair pool := by
  rw [Multiset.le_iff_count]
  intro x
  obtain ⟨bx, vx⟩ := x
  by_cases hbx : bx = qb
  · subst hbx
    -- count in the mapped bucket = the bucket's payload multiplicity
    have hcount1 : Multiset.count ((bx, vx) : Ballot nP × ALog P nP)
        (Multiset.ofList (vs.map (fun v => (bx, v))))
        = vs.countP (fun w => decide (w = vx)) := by
      rw [count_ofList, List.countP_map]
      refine List.countP_congr (fun w _ => ?_)
      constructor
      · intro h
        have := of_decide_eq_true h
        exact decide_eq_true (congrArg Prod.snd this)
      · intro h
        exact decide_eq_true (by rw [of_decide_eq_true h])
    rw [hcount1]
    -- the bucket lives in some selection prefix
    unfold pP1bViews at ht hmem
    have ht0 : t < (prefixCuts (selectOrder
        (Multiset.filterMap p1bOkPair pool) dOrd) 0 dSnap).length := by
      rwa [List.length_map] at ht
    rw [List.getElem_map] at hmem
    have hcnt := foldEarlyStop_count hmem vx
    obtain ⟨k, hk⟩ := prefixCuts_mem_take (List.getElem_mem ht0)
    rw [hk] at hcnt
    -- prefix counts under selection counts under pool counts
    have hsel : ((selectOrder (Multiset.filterMap p1bOkPair pool)
        dOrd).take k).countP
          (fun e => decide (e.1 = bx) && decide (e.2 = vx))
        ≤ (selectOrder (Multiset.filterMap p1bOkPair pool)
            dOrd).countP
          (fun e => decide (e.1 = bx) && decide (e.2 = vx)) :=
      (List.take_sublist ..).countP_le
    have hpair : (selectOrder (Multiset.filterMap p1bOkPair pool)
        dOrd).countP
          (fun e => decide (e.1 = bx) && decide (e.2 = vx))
        = Multiset.count ((bx, vx) : Ballot nP × ALog P nP)
            (Multiset.ofList (selectOrder
              (Multiset.filterMap p1bOkPair pool) dOrd)) := by
      rw [count_ofList]
      refine List.countP_congr (fun e _ => ?_)
      constructor
      · intro h
        rw [Bool.and_eq_true, decide_eq_true_iff, decide_eq_true_iff]
          at h
        exact decide_eq_true (Prod.ext h.1 h.2)
      · intro h
        have he := of_decide_eq_true h
        rw [Bool.and_eq_true, decide_eq_true_iff, decide_eq_true_iff]
        exact ⟨congrArg Prod.fst he, congrArg Prod.snd he⟩
    have hle := Multiset.count_le_of_le
      ((bx, vx) : Ballot nP × ALog P nP)
      (selectOrder_subpool (pool := Multiset.filterMap p1bOkPair pool)
        (d := dOrd))
    rw [hpair] at hsel
    omega
  · -- foreign ballots do not occur in the mapped bucket
    have hzero : Multiset.count ((bx, vx) : Ballot nP × ALog P nP)
        (Multiset.ofList (vs.map (fun v => (qb, v)))) = 0 := by
      rw [Multiset.count_eq_zero]
      intro hc
      obtain ⟨w, -, hw⟩ := List.mem_map.mp (Multiset.mem_coe.mp hc)
      exact hbx (congrArg Prod.fst hw).symm
    omega

/-! ## Reign stability (the fabricated-reign regress, FINDINGS D21)

What `p_p1b` requires of its caller, named per guarantee: the election
loop discharges these from its own wires — `p_has_largest_ballot` is
identically true, ballots are owned and `num`-monotone, and every
`Ok`-promised ballot was **solicited at a flag-`false` tick** (the
feedback fact through the election cycle: `p_to_acceptors_p1a` is gated
on the trigger, which fires only at non-leader ticks). -/
/-- **No flag-`false` tick sees its own ballot's bucket full** — the
masking regress: a mask needs a strictly higher full own bucket, whose
solicitation tick is strictly later and again masked; the ascent
exhausts the trace. -/
theorem pP1b_no_false_full {nP : Nat} (quorumSize : Nat)
    (hq1 : 1 ≤ quorumSize) (pool : Multiset (P1b P nP))
    (pb : Trace (Ballot nP)) (phl : Trace Bool)
    (dOrd : List (Ballot nP × ALog P nP)) (dSnap : List Nat) (me : Fin nP)
    (req : PP1bRequires nP quorumSize pool pb phl dOrd dSnap me) :
    ∀ (n u : Nat)
      (hu : u < (pP1bFlags nP quorumSize pool pb phl dOrd dSnap).length),
      (pP1bFlags nP quorumSize pool pb phl dOrd dSnap).length - u ≤ n →
      (pP1bFlags nP quorumSize pool pb phl dOrd dSnap)[u]'hu = false →
      ∀ (hv : u < (pP1bViews nP quorumSize pool dOrd dSnap).length)
        (hb : u < pb.length) (vs : List (ALog P nP)),
        (pb[u]'hb, vs) ∈ (pP1bViews nP quorumSize pool dOrd dSnap)[u]'hv →
        quorumSize ≤ vs.length → False := by
  intro n
  induction n with
  | zero => intro u hu hn; omega
  | succ n ih =>
    intro u hu hn hfalse hv hb vs hmem hfull
    obtain ⟨hv₀, hb₀, hl₀, heq⟩ :=
      pP1bFlags_getElem quorumSize pool pb phl dOrd dSnap hu
    have hphl : phl[u]'hl₀ = true :=
      req.has_largest _ (List.getElem_mem hl₀)
    rw [heq, hphl, Bool.and_true] at hfalse
    -- the max full bucket exists and dominates ours, but is not ours
    obtain ⟨r, hmax, hble⟩ :=
      p1bMaxQuorumBallot_ge_full quorumSize hmem hfull
    obtain ⟨rb, rlogs⟩ := r
    have hne : rb ≠ pb[u]'hb := by
      intro hc
      have hsome : pP1bQuorum quorumSize
          ((pP1bViews nP quorumSize pool dOrd dSnap)[u]'hv)
          (pb[u]'hb) = some rlogs := by
        unfold pP1bQuorum
        rw [hmax]
        dsimp only
        rw [if_pos hc]
      rw [hsome] at hfalse
      cases hfalse
    have hblt : (pb[u]'hb).blt rb = true :=
      Ballot.blt_of_ble_ne hble (fun hc => hne hc.symm)
    -- the mask is full and realized
    have hrmem : ((rb, rlogs) : Ballot nP × List (ALog P nP))
        ∈ (pP1bViews nP quorumSize pool dOrd dSnap)[u]'hv := by
      have := p1bMaxQuorumBallot_mem_self hmax
      exact this
    have hrfull : quorumSize ≤ rlogs.length :=
      p1bMaxQuorumBallot_holds_quorum quorumSize _ hmax
    have hrlen : rlogs.length = quorumSize :=
      Nat.le_antisymm
        (pP1bViews_length_le quorumSize hq1 pool dOrd dSnap hv hrmem)
        hrfull
    -- its ballot was promised, hence solicited at a flag-false tick
    obtain ⟨v, hvr⟩ := List.exists_mem_of_length_pos
      (Nat.lt_of_lt_of_le hq1 hrfull)
    obtain ⟨m, hm, hmb, hmres⟩ := pP1bViews_entry_ok quorumSize pool dOrd
      dSnap hv hrmem v hvr
    obtain ⟨u', hu', hb', hpbu', hflagu'⟩ :=
      req.solicited_at_follower m hm ⟨v, hmres⟩
    rw [hmb] at hpbu'
    -- ownership forces a strict `num` ascent, so `u < u'`
    have hpid : rb.proposerId = (pb[u]'hb).proposerId := by
      rw [← hpbu']
      rw [req.ballot_own _ (List.getElem_mem hb'),
        req.ballot_own _ (List.getElem_mem hb)]
    have hnum : (pb[u]'hb).num < rb.num := by
      rcases Ballot.blt_iff.mp hblt with hlt | ⟨-, hpidlt⟩
      · exact hlt
      · rw [hpid] at hpidlt
        omega
    have huu' : u < u' := by
      by_contra hc
      have hle : u' ≤ u := Nat.le_of_not_lt hc
      have hmono := req.ballot_mono hle hb
      have hnum' : (pb[u']'(Nat.lt_of_le_of_lt hle hb)).num = rb.num := by
        have he : pb[u']'(Nat.lt_of_le_of_lt hle hb) = rb := hpbu'
        rw [he]
      omega
    -- the mask carries forward to its own solicitation tick
    obtain ⟨hv', hbx, hlx, -⟩ :=
      pP1bFlags_getElem quorumSize pool pb phl dOrd dSnap hu'
    have hcarry := pP1bViews_full_carry quorumSize pool dOrd dSnap
      (Nat.le_of_lt huu') hv' hrmem hrlen
    -- recurse strictly later
    refine ih u' hu' (by omega) hflagu' hv' hb' rlogs ?_ hrfull
    rw [hpbu']
    exact hcarry

/-- **Ballot stability along reigns**: consecutive leader ticks keep the
ballot — a fabricated reign cannot bootstrap (FINDINGS D21). -/
theorem pP1b_ballot_stable {nP : Nat} (quorumSize : Nat)
    (hq1 : 1 ≤ quorumSize) (pool : Multiset (P1b P nP))
    (pb : Trace (Ballot nP)) (phl : Trace Bool)
    (dOrd : List (Ballot nP × ALog P nP)) (dSnap : List Nat) (me : Fin nP)
    (req : PP1bRequires nP quorumSize pool pb phl dOrd dSnap me)
    {t : Nat}
    (ht1 : t + 1 < (pP1bFlags nP quorumSize pool pb phl dOrd dSnap).length)
    (h1 : (pP1bFlags nP quorumSize pool pb phl dOrd dSnap)[t + 1]'ht1
      = true)
    (h0 : (pP1bFlags nP quorumSize pool pb phl dOrd
      dSnap)[t]'(Nat.lt_of_succ_lt ht1) = true) :
    ∃ hb1 : t + 1 < pb.length,
      pb[t + 1]'hb1 = pb[t]'(Nat.lt_of_succ_lt hb1) := by
  obtain ⟨hv1, hb1, hl1, heq1⟩ :=
    pP1bFlags_getElem quorumSize pool pb phl dOrd dSnap ht1
  refine ⟨hb1, ?_⟩
  by_contra hne
  -- `num`s ascend; distinct own ballots ascend strictly
  have hmono : (pb[t]'(Nat.lt_of_succ_lt hb1)).num
      ≤ (pb[t + 1]'hb1).num := req.ballot_mono (Nat.le_succ t) hb1
  have hnum : (pb[t]'(Nat.lt_of_succ_lt hb1)).num
      < (pb[t + 1]'hb1).num := by
    rcases Nat.lt_or_ge (pb[t]'(Nat.lt_of_succ_lt hb1)).num
        (pb[t + 1]'hb1).num with hlt | hge
    · exact hlt
    · exfalso
      have hnume : (pb[t + 1]'hb1).num
          = (pb[t]'(Nat.lt_of_succ_lt hb1)).num := by omega
      have hpide : (pb[t + 1]'hb1).proposerId
          = (pb[t]'(Nat.lt_of_succ_lt hb1)).proposerId := by
        rw [req.ballot_own _ (List.getElem_mem hb1),
          req.ballot_own _ (List.getElem_mem (Nat.lt_of_succ_lt hb1))]
      exact hne (Ballot.key_injective (by
        unfold Ballot.key
        rw [hnume, hpide]))
  -- the reign's quorum is realized at `t + 1`
  rw [heq1] at h1
  rw [Bool.and_eq_true] at h1
  obtain ⟨qlogs, hq⟩ := Option.isSome_iff_exists.mp h1.1
  have hqm : p1bMaxQuorumBallot quorumSize
      ((pP1bViews nP quorumSize pool dOrd dSnap)[t + 1]'hv1)
      = some (pb[t + 1]'hb1, qlogs) := by
    unfold pP1bQuorum at hq
    cases hmax : p1bMaxQuorumBallot quorumSize
        ((pP1bViews nP quorumSize pool dOrd dSnap)[t + 1]'hv1) with
    | none => rw [hmax] at hq; cases hq
    | some r =>
      obtain ⟨rb, rlogs⟩ := r
      rw [hmax] at hq
      dsimp only at hq
      by_cases hrb : rb = pb[t + 1]'hb1
      · rw [if_pos hrb] at hq
        injection hq with hq'
        rw [← hrb, ← hq']
      · rw [if_neg hrb] at hq
        cases hq
  have hmem : ((pb[t + 1]'hb1, qlogs) :
      Ballot nP × List (ALog P nP))
      ∈ (pP1bViews nP quorumSize pool dOrd dSnap)[t + 1]'hv1 :=
    p1bMaxQuorumBallot_mem_self hqm
  have hfull : quorumSize ≤ qlogs.length :=
    p1bMaxQuorumBallot_holds_quorum quorumSize _ hqm
  have hlen : qlogs.length = quorumSize :=
    Nat.le_antisymm
      (pP1bViews_length_le quorumSize hq1 pool dOrd dSnap hv1 hmem) hfull
  -- the reign ballot was promised, hence solicited flag-false
  obtain ⟨v, hvq⟩ := List.exists_mem_of_length_pos
    (Nat.lt_of_lt_of_le hq1 hfull)
  obtain ⟨m, hm, hmb, hmres⟩ := pP1bViews_entry_ok quorumSize pool dOrd
    dSnap hv1 hmem v hvq
  obtain ⟨u, hu, hbu, hpbu, hflagu⟩ :=
    req.solicited_at_follower m hm ⟨v, hmres⟩
  rw [hmb] at hpbu
  -- the solicitation is not before the reign …
  have hut : t + 1 ≤ u := by
    by_contra hc
    have hle : u ≤ t := by omega
    have := req.ballot_mono hle (Nat.lt_of_succ_lt hb1)
    rw [hpbu] at this
    omega
  -- … and not at it, so strictly after: the mask regress kills it
  have hut' : t + 1 < u := by
    rcases Nat.lt_or_eq_of_le hut with hlt | heqe
    · exact hlt
    · exfalso
      subst heqe
      have hfl : (pP1bFlags nP quorumSize pool pb phl dOrd
          dSnap)[t + 1]'ht1 = false := hflagu
      rw [heq1, h1.1, Bool.true_and] at hfl
      rw [req.has_largest _ (List.getElem_mem hl1)] at hfl
      cases hfl
  obtain ⟨hv', hbx, hlx, -⟩ :=
    pP1bFlags_getElem quorumSize pool pb phl dOrd dSnap hu
  have hcarry := pP1bViews_full_carry quorumSize pool dOrd dSnap
    (Nat.le_of_lt hut') hv' hmem hlen
  refine pP1b_no_false_full quorumSize hq1 pool pb phl dOrd dSnap me req
    (pP1bFlags nP quorumSize pool pb phl dOrd dSnap).length u hu
    (by omega) hflagu hv' hbu qlogs ?_ hfull
  rw [hpbu]
  exact hcarry


/-- Bucket uniqueness lifted to realized views: one bucket per ballot
per tick. -/
theorem pP1bViews_bucket_unique {nP : Nat} {quorumSize : Nat}
    {pool : Multiset (P1b P nP)}
    {dOrd : List (Ballot nP × ALog P nP)} {dSnap : List Nat} {t : Nat}
    (ht : t < (pP1bViews nP quorumSize pool dOrd dSnap).length)
    {qb : Ballot nP} {vs vs' : List (ALog P nP)}
    (h : (qb, vs) ∈ (pP1bViews nP quorumSize pool dOrd dSnap)[t]'ht)
    (h' : (qb, vs') ∈ (pP1bViews nP quorumSize pool dOrd dSnap)[t]'ht) :
    vs = vs' := by
  unfold pP1bViews at ht h h'
  rw [List.getElem_map] at h h'
  exact foldEarlyStop_bucket_unique h h'

/-- **The leader bucket**: a true flag surfaces the tick's own bucket in
the tick's realized view — the quorum answers with the bucket at
`p_ballot`, exactly `quorumSize` logs strong (`≤` from the insert cap,
`≥` from the stop condition), and `p_has_largest_ballot` is up. -/
theorem pP1b_leader_bucket {nP : Nat} (quorumSize : Nat)
    (hq1 : 1 ≤ quorumSize) (pool : Multiset (P1b P nP))
    (pb : Trace (Ballot nP)) (phl : Trace Bool)
    (dOrd : List (Ballot nP × ALog P nP)) (dSnap : List Nat) {u : Nat}
    (hu : u < (pP1bFlags nP quorumSize pool pb phl dOrd dSnap).length)
    (htrue : (pP1bFlags nP quorumSize pool pb phl dOrd dSnap)[u]'hu
      = true) :
    ∃ (hv : u < (pP1bViews nP quorumSize pool dOrd dSnap).length)
      (hb : u < pb.length) (hl : u < phl.length)
      (qlogs : List (ALog P nP)),
      pP1bQuorum quorumSize
          ((pP1bViews nP quorumSize pool dOrd dSnap)[u]'hv) (pb[u]'hb)
        = some qlogs
      ∧ (pb[u]'hb, qlogs)
          ∈ (pP1bViews nP quorumSize pool dOrd dSnap)[u]'hv
      ∧ qlogs.length = quorumSize
      ∧ phl[u]'hl = true := by
  obtain ⟨hv, hb, hl, heq⟩ :=
    pP1bFlags_getElem quorumSize pool pb phl dOrd dSnap hu
  rw [heq] at htrue
  simp only [Bool.and_eq_true, Option.isSome_iff_exists] at htrue
  obtain ⟨⟨qlogs, hq⟩, hphl⟩ := htrue
  refine ⟨hv, hb, hl, qlogs, hq, ?_⟩
  unfold pP1bQuorum at hq
  cases hmax : p1bMaxQuorumBallot quorumSize
      ((pP1bViews nP quorumSize pool dOrd dSnap)[u]'hv) with
  | none =>
    rw [hmax] at hq
    cases hq
  | some r =>
    obtain ⟨rb, rlogs⟩ := r
    rw [hmax] at hq
    dsimp only at hq
    by_cases hrb : rb = pb[u]'hb
    · rw [if_pos hrb] at hq
      injection hq with hq'
      subst hq'
      have hmem := p1bMaxQuorumBallot_mem_self hmax
      rw [hrb] at hmem
      have hge := p1bMaxQuorumBallot_holds_quorum quorumSize _ hmax
      have hle := pP1bViews_length_le quorumSize hq1 pool dOrd dSnap hv
        hmem
      exact ⟨hmem, Nat.le_antisymm hle hge, hphl⟩
    · rw [if_neg hrb] at hq
      cases hq

/-- **Quorum pinning across leader ticks**: two true-flag ticks at the
same ballot answer with the same bucket — full buckets freeze
(`pP1bViews_full_carry`) and keys are unique per tick. -/
theorem pP1b_quorum_pinned {nP : Nat} (quorumSize : Nat)
    (hq1 : 1 ≤ quorumSize) (pool : Multiset (P1b P nP))
    (pb : Trace (Ballot nP)) (phl : Trace Bool)
    (dOrd : List (Ballot nP × ALog P nP)) (dSnap : List Nat)
    {t t' : Nat} (h : t ≤ t')
    (ht : t < (pP1bFlags nP quorumSize pool pb phl dOrd dSnap).length)
    (ht' : t' < (pP1bFlags nP quorumSize pool pb phl dOrd dSnap).length)
    (htrue : (pP1bFlags nP quorumSize pool pb phl dOrd dSnap)[t]'ht
      = true)
    (htrue' : (pP1bFlags nP quorumSize pool pb phl dOrd dSnap)[t']'ht'
      = true)
    {hb : t < pb.length} {hb' : t' < pb.length}
    (hpb : pb[t]'hb = pb[t']'hb') :
    ∃ (hv : t < (pP1bViews nP quorumSize pool dOrd dSnap).length)
      (hv' : t' < (pP1bViews nP quorumSize pool dOrd dSnap).length)
      (qlogs : List (ALog P nP)),
      pP1bQuorum quorumSize
          ((pP1bViews nP quorumSize pool dOrd dSnap)[t]'hv) (pb[t]'hb)
        = some qlogs
      ∧ pP1bQuorum quorumSize
          ((pP1bViews nP quorumSize pool dOrd dSnap)[t']'hv')
          (pb[t']'hb')
        = some qlogs := by
  obtain ⟨hv, hb0, hl, qlogs, hq, hmem, hlen, -⟩ :=
    pP1b_leader_bucket quorumSize hq1 pool pb phl dOrd dSnap ht htrue
  obtain ⟨hv', hb0', hl', qlogs', hq', hmem', hlen', -⟩ :=
    pP1b_leader_bucket quorumSize hq1 pool pb phl dOrd dSnap ht' htrue'
  -- align the getElem proof witnesses
  have hq2 : pP1bQuorum quorumSize
      ((pP1bViews nP quorumSize pool dOrd dSnap)[t]'hv) (pb[t]'hb)
      = some qlogs := hq
  have hq2' : pP1bQuorum quorumSize
      ((pP1bViews nP quorumSize pool dOrd dSnap)[t']'hv')
      (pb[t']'hb') = some qlogs' := hq'
  -- carry `t`'s frozen bucket to `t'` and identify by key
  have hcarry : ((pb[t]'hb, qlogs) : Ballot nP × List (ALog P nP))
      ∈ (pP1bViews nP quorumSize pool dOrd dSnap)[t']'hv' :=
    pP1bViews_full_carry quorumSize pool dOrd dSnap h hv'
      (show ((pb[t]'hb, qlogs) : Ballot nP × List (ALog P nP))
        ∈ (pP1bViews nP quorumSize pool dOrd
          dSnap)[t]'(Nat.lt_of_le_of_lt h hv') from hmem) hlen
  have hmem2' : ((pb[t]'hb, qlogs') : Ballot nP × List (ALog P nP))
      ∈ (pP1bViews nP quorumSize pool dOrd dSnap)[t']'hv' := by
    rw [hpb]
    exact hmem'
  have hqq : qlogs = qlogs' :=
    pP1bViews_bucket_unique hv' hcarry hmem2'
  exact ⟨hv, hv', qlogs, hq2, by rw [hqq]; exact hq2'⟩

end HydroV2
