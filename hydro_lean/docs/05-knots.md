# 05 — Knots: `fix … complete`, the Kleene chain, and K4

Rust closes a cycle with `forward_ref`/`complete_cycle`. `paxos_core`
(`paxos.rs:136–246`) has two such cycles tying `leader_election` to `sequence_payload`:
the acceptor log `a_log` and the sequencing max-ballots. The Lean mirror is a `fix …
complete` block (`Hydro/Paxos/PaxosCore.lean:136–306`); the Paxos safety argument — K4,
quorum intersection plus the provenance regress — is the block's `invariant` clause.

## The construct

```lean
-- Hydro/Paxos/PaxosCore.lean:133–139
  -- paxos.rs:141–148: the `a_log` and `sequencing_max_ballot`
  -- forward_ref cycles, closed mutually (`a_log` re-closed per
  -- sequencing iterate — Rust's `snapshot_atomic` write-before-ack)
  fix (a_log : H.Ticked acc (ALog P (mem prop)))
      (sequencing_max_ballots : H.Stream prop (Ballot (mem prop))
        .noOrder .exactlyOnce)
      via (dec.fuelALog, dec.fuelSeqMax)
```

`fix (w₁ : τ₁) … (wₙ : τₙ) via (f₁, …, fₙ) := body … complete (e₁, …, eₙ); rest` closes
the mutual cycle `(w₁, …, wₙ) = body` — each `wᵢ` is in scope in the body, and
`complete` supplies the values that close them (Rust's `complete_cycle`). The `fᵢ` are
`FixDec` fuels: at `Values` the knot is a bounded Kleene iteration (`UnfoldFuel`), one
of the decision inputs; at `SchedSem` the knot is the Kleene **diagonal** — cycle
unfolding *is* step progression, no fuel (chapter 06).

The body (`PaxosCore.lean:283–303`) is the Rust text:

```lean
        let le := leader_election H variant prop acc (f + 1) (2 * f + 1)
          dec.le sched.le sequencing_max_ballots a_log
        ghost have hle := leader_election.ensures variant prop acc (f + 1) (2 * f + 1)
          dec.le sched.le sequencing_max_ballots a_log
        let just_became_leader := H.mapTick (H.zipTick le.2.1 (H.defer_tick false le.2.1))
          (fun _me x => x.1 && !x.2)
        let sp := sequence_payload H variant prop acc c_to_proposers a_checkpoint
          le.1 le.2.1 le.2.2.1 f le.2.2.2 dec.sp sched.sp
        ghost have hsp := sequence_payload.ensures variant prop acc c_to_proposers
          a_checkpoint le.1 le.2.1 le.2.2.1 f le.2.2.2 dec.sp sched.sp
      -- a_log_complete_cycle.complete(a_log);
      -- sequencing_max_ballot_complete_cycle.complete(seq_max_ballots)
      complete (sp.2.1, sp.2.2)
```

Two module calls, two `ghost have` faces, `complete`. The `ghost have`s inside a `fix`
body **replay in the prove leg at the closed wires** — so `hle`/`hsp` below the block
are the final-stage contracts, obtained by composition, with no re-spelling of the
module applications (D58).

## What the elaborator does (`Hydro/HydroDef.lean:98–`, `HydroGenKnot.lean:1307–`)

- **Bekić decomposition**: the mutual cycle becomes a cascade of single knots, each
  hoisted to a top-level constant `paxos_core.a_log`, `paxos_core.sequencing_max_ballots`
  with a folded `@[reducible]` body. This is a kernel-cost decision, not a cosmetic one:
  defeq through `k` nested knots is exponential in `k` (D40), so every proof is
  module-sized and every artifact is per knot.
- **The Kleene chain, generated** (`genKnotStages`): `paxos_core.a_log.stages` (the
  `Values` iterates), `stages_zero` (stage 0 is the seed), `stages_succ` (one knot step),
  `stages_fix` (the knot at `Values` *is* its stage at the fuel), `stages_mono` (deeper
  stages extend shallower ones — the chain). Any-body truths, never re-proven.
- **The invariant clause**: `invariant a_log (le sp le' sp') => P, base := …, step :=
  …` packages a **bounded-chain induction**: `base` is `P` at stage 0, `step` is
  `P (stage w) → P (stage (w+1))` with the body's faces and the chain orders handed in
  as hypotheses; the construct proves `paxos_core.a_log.inv` and lands it in the prove
  leg as `ha_log_inv` (D57, D58). No proof outside the block mentions stages.
- The knot pipeline (`runKnotStackT`) also generates the corner artifacts
  (`paxos_core.a_log_co_sr₁`, `_co_rr₁`, `_co_wf₁`) that chapter 06 consumes.

## K4, read line by line

The predicate (`PaxosCore.lean:140–157`). `le`/`sp` are *this stage's* runs (the body's
own `let`s), `le'`/`sp'` the **closed knot's**:

```lean
      invariant a_log (le sp le' sp') =>
        variant = .guarded → mem acc ≤ 2 * f + 1 →
        ∀ (slot : Nat) (b₁ b₂ : Ballot (mem prop)) (val₁ v₂ : Option P) (i₂ : Fin (mem prop)),
          SPChosen f (fun j => a_checkpoint j (dec.sp.ap2.ckSnap j)) le'.2.2.2 sp'.2.1 slot b₁ →
          (∀ (j : Fin (mem acc)) {t : Nat} {lg : ALog P (mem prop)} {e : LogValue P (mem prop)},
            (sp'.2.1 j)[t]? = some lg → (slot, e) ∈ lg.2 → e.ballot = b₁ → e.value = val₁) →
          b₁.blt b₂ = true →
          LeaderOpen (le.1 i₂) (le.2.1 i₂) (le.2.2.1 i₂) b₂ slot v₂ →
          v₂ = val₁,
```

In words: if `(slot, b₁)` is **chosen** at the closed knot (`SPChosen`: `f+1` distinct
acceptors' logs carry it) and every closed-knot log entry at `(slot, b₁)` carries
`val₁`, then any value this stage's leader **opens** at a higher ballot `b₂`
(`LeaderOpen`: a leader tick of `b₂` whose input view characterizes `v₂`) is `val₁`.
Everything is stated over **log entries** — outputs of `sequence_payload`'s face — not
over the sequencer's sent traffic, which is what let the old `SPEmission` mirror be
deleted (D64; `../DOCTRINE.md` R3).

`base` (`:158–171`): at stage 0 nothing has been opened — no acceptor has ticked, so a
leader tick's promise quorum cannot exist (`hle.view_promise` gives a promiser at a
tick index into an empty trace; `simp at htj` closes it).

`step` (`:172–281`), the protocol argument, with its hypotheses named by the construct:
`hw`/`hbw`/`htop` (the chain orders: this stage ≤ next ≤ top), `hle`/`hsp`/`hle_step`/
`hsp_step`/`hle'`/`hsp'` (the faces at the three stages), `ih`.

1. *Lift this stage's log to the top* — `hpublift : ∀ jj, sp.2.1 jj <+: sp'.2.1 jj :=
   fun jj => (hbw jj).trans (htop jj)` — the chain order, nothing else.
2. *Lift this stage's max wire to the top* — `hblift₄ := leader_election_mono₄ … hbw`:
   the **generated monotonicity** of `leader_election`, the "re-execution with larger
   inputs" fact (chapter 03), not a face.
3. *Open the leader tick* — `obtain ⟨t, view, hl, hb, hv, hchar⟩ := hopen`; get `f+1`
   distinct promisers at it from `hle_step.providers`; get `f+1` distinct voters for the
   chosen key from `hch`.
4. **Quorum intersection** — `obtain ⟨j, hjS, hjC⟩ := nodup_inter_of_length hSnd hCnd (by
   omega)`: two sets of `f+1` distinct acceptors among `≤ 2f+1` share one (`Types.lean`,
   the one pigeonhole lemma).
5. *The vote precedes the promise* on acceptor `j`'s ascending max wire —
   `hle'.max_mono` (an `AP1Ensures`-derived face of the election) with the ballot order
   `Ballot.ble_blt_asymm`.
6. *The promise payload is this stage's published log entry* (`hpayeq`), the vote's
   coverage is on it (`hsp'.log_covers_mono`), so the covering entry is in the opening
   tick's input view (`rcEntries`).
7. *The opened value is the view's champion* at the slot (`hchar`); the champion's
   group agrees (`logView_entry_value` + `hsp.log_entry_agree`).
8. **Dichotomy at the champion's ballot** (`Ballot.eq_or_blt_of_ble`): equal to `b₁` →
   its entry lifted to the top carries `val₁` by hypothesis; strictly higher → this
   stage's leader opened it (`hsp.log_entry_open`), so the **induction hypothesis**
   applies and gives `val₁`.

~110 lines, all of it Paxos. Nothing about stages, scans, fuels or the machine.

## The prove leg: contract composition

```lean
-- Hydro/Paxos/PaxosCore.lean:312–331
      prove
        slot_functional := fun hvar hnA => by
          subst hvar
          have hI := ha_log_inv rfl hnA                 -- the packaged induction, closed wires
          have hreq' := hle.discipline (by omega)       -- the election face, final stage
          intro i i' slot v v' h h'
          by_contra hne
          obtain ⟨b, b', hbb, W, W'⟩ := hsp.commit_distinct hreq' rfl h h' hne
          cases hc : b.blt b' with
          | true => exact hne (hI slot b b' v v' i' W.chosen W.entries_agree hc W'.opened).symm
          | false =>
            have hlt := Ballot.blt_of_ble_ne (Ballot.ble_of_not_blt hc) (Ne.symm hbb)
            exact hne (hI slot b' b v' v i W'.chosen W'.entries_agree hlt W.opened)
```

Two commits of different values at one slot have distinct witness ballots
(`hsp.commit_distinct`, a face of `sequence_payload`); the lower is chosen and agreed
by its entries, the higher was opened by its leader — K4 at the closed knot (`hI`) says
the opened value is the chosen one. Done. The whole safety headline is: one face of
`sequence_payload`, one face of `leader_election`, the invariant. This is what
"proofs decomposed along the program; the outside proof is contract composition only"
means (`FINDINGS.md` D57).

## History, briefly

K4 used to be a 560-line separate file (`PaxosCoreLemmas.lean`) re-deriving stage
vocabulary by hand (D57 deleted it); then an invariant over the sequencer's *sent*
traffic, which needed a pure mirror of the sequencing pipeline to transport across
stages (D64 restated it over entries and deleted the mirror). The current form is the
one that ports to Verus as a loop invariant on the `forward_ref` cycle
(`../DOCTRINE.md` §table).
