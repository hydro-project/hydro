# 04 — Ghosts and proofs: where the proof lives in the program

Proof text in a `hydro def` sits at the program point it is about (`../DOCTRINE.md` R2).
There are four places it can sit: an `invariant` clause on a `tick`/`fix` block with its
`prove init/tick` (or `base/step`) obligations; `ghost have`/`ghost obtain` lines after
the wire they decode; the trailing `prove` block discharging the contract's fields; and,
for genuinely pure mathematics, `Trace.lean`/`Grades.lean`. Nothing else — no lemma
file, no mirror of the body.

This chapter reads `collect_quorum`'s proof text (`Hydro/Std/Quorum.lean:718–897`) and
then the shared once-per-ballot invariant `OnceInv` as `sequence_payload` uses it.

## 1. The construct's gifts

When the `tick` block elaborates (`Hydro/HydroTick.lean`), it emits — for an output
`just_reached_quorum` — a handful of facts your proof may cite by name:

| name | what it says |
|---|---|
| `just_reached_quorum_step` | the per-tick body as a function `me st inpt ↦ (next states, emissions)` — reading it is `simp only [just_reached_quorum_step, den]` |
| `hjust_reached_quorum_run` | the output wire **is** the structural fold over the member's ticks (`scanAcrossTicksTrace …`) |
| `hjust_reached_quorum_at` | the option-indexed tick reader: `(out i)[n]? = some e ↔ ∃ inputs read at n, e = (step … ).2` |
| `hjust_reached_quorum_reg` | (stateful blocks) the register, ∃-abstracted: seed, read, step, stall — one name for "the state before tick `n`"; `collect_quorum_with_response` cites its `hquorums_reg` |
| `hjust_reached_quorum_inv`, `hjust_reached_quorum_inv_take` | the invariant at the whole run / at every prefix (only with an `invariant` clause) |

These are *any-body truths* (`../DOCTRINE.md` R6): true for every `tick` block, so the
elaborator proves them once, generically (`scanAcrossTicks_invariant` and the
`TickReads` lemmas in `Trace.lean`), and the module never re-derives them.

## 2. The loop invariant and its two obligations

From chapter 01, the clause:

```lean
-- Hydro/Std/Quorum.lean:678–682
      (invariant ((just_reached_quorum : List (Multiset K))
          (not_all : Multiset (K × Except E Unit)) (min_but_not_max : Multiset K)
          (new_inputs : Trace (Multiset (K × Except E Unit)))) =>
        CQRegInv min max not_all min_but_not_max
          ((new_inputs.take just_reached_quorum.length).sum)) :=
```

The predicate is in **loop-invariant normal form**: a relation between the emissions so
far (`just_reached_quorum`, a list), the current registers (`not_all`,
`min_but_not_max`), and a spectator input (`new_inputs`) — no history indices, no
"∀ t < n". `CQRegInv` (`Quorum.lean`, before the def — the one allowed pre-def proof
budget) says the registers carry exactly the right residue of the responses consumed so
far. This is a Verus loop invariant, written where Verus would write it.

The obligations trail the block:

```lean
-- Hydro/Std/Quorum.lean:720–735
      prove init := fun _i => by
          simp only [List.length_nil, List.take_zero, List.sum_nil, ValuesTick.seed_pair,
            ValuesTick.seed_stream, den]
          exact CQRegInv.init,
        tick := fun i n out st b_t hb hlen ih => by
          simp only [List.append_eq, List.length_append, List.length_singleton, hlen,
            List.take_add_one, hb, Option.toList_some, List.sum_append, List.sum_singleton] at ih ⊢
          simp only [just_reached_quorum_step, den]
          by_cases hmm : max = min
          · simp only [if_pos hmm, cq_reached_eq, …]
            exact CQRegInv.step_eq hmm.symm ih
          · simp only [if_neg hmm, cq_reached_eq, cq_keys_filter_eq, …]
            exact CQRegInv.step_ne (Ne.symm hmm) ih;
```

`init`: the invariant at the empty run (seeds are the grade's bottom). `tick`: **one
loop-body step** — given this tick's input `b_t` read at `n` (`hb`), the emissions so far
of length `n` (`hlen`), and the invariant before (`ih`), the invariant after. The body
is opened with `simp only [just_reached_quorum_step, den]` — the Rust lines at the
denotation — then Rust's `if max == min` becomes a `by_cases`, and each branch is one
step lemma of `CQRegInv`. Twelve lines; the construct supplies the induction.

## 3. Ghosts: decoding one tick, after the block

```lean
-- Hydro/Std/Quorum.lean:742–760 (abridged)
  ghost have htick : ∀ (i : Fin (mem ℓ)) (n : Nat) (e : Multiset K),
      (just_reached_quorum i)[n]? = some e →
      ∃ (b_t : …) (S : …),
        (new_inputs i)[n]? = some b_t
        ∧ CQRegInv min max S.1 S.2 ((new_inputs i).take n).sum
        ∧ e = (if max = min then … else …) := fun i n e he => by
    have hn : n < (just_reached_quorum i).length := Trace.read_lt he
    obtain ⟨b_t, hb, rfl⟩ := (hjust_reached_quorum_at i n e).mp he
    have h := hjust_reached_quorum_inv_take i n
    …
    simp only [just_reached_quorum_step, den]
```

A `ghost have` is a `have` that exists only in the proof leg: the computational program
(what `Eager` runs) never sees it. It cites the construct's readers (`_at`, `_inv_take`)
and the body under `den` — and it refers to the wires by their **program names**
(`just_reached_quorum`, `new_inputs`). That is `../DOCTRINE.md` R4: if you find yourself
writing out a wire's unfolded denotation, or aliasing one with `ghost let`, the face or
binder that should have given you the name is wrong — fix that instead.

`hcount_tick` (`Quorum.lean:767–826`, ~60 lines) is the one genuinely protocol-specific
fact: on one tick, key `k` is emitted iff it crossed `min` on this tick. `hcount`
(`:829–860`) sums it over a prefix by induction on `n`. Both read the program through
`htick`; neither mentions the scan, the state tuple, or any mirror.

Other ghost forms: `ghost obtain ⟨x, hx⟩ := …` (destructure a face or a `_reg`;
**name the witness** — `⟨-, …⟩` on an ∃-witness silently clears every dependent
hypothesis, `../ENGINE_NOTES.md`), `ghost witness e` (supply a `∃` of the module's own
face), `ghost intro`/`ghost subst` (open a face stated as an implication).

## 4. The `prove` block: the face, field by field

```lean
-- Hydro/Std/Quorum.lean:867–897
  prove
    emit_sound := fun i k hk => by
      -- `k` left on some tick: that tick's window held `min` `Ok`s for it,
      -- and the window sits below the consumed pool
      obtain ⟨e, he, hke⟩ := mem_list_sum.mp hk
      obtain ⟨n, hn⟩ := List.mem_iff_getElem?.mp he
      obtain ⟨b_t, S, hb, hS, rfl⟩ := htick i n e hn
      …
      exact le_trans hok (cqOkCount_mono hwin k),
    emit_count := fun i k h1 hmx hcap => by
      show ((just_reached_quorum i).sum).count k = _
      have hlen : (just_reached_quorum i).length = (new_inputs i).length := by
        rw [hjust_reached_quorum_run i, scanAcrossTicksTrace_length]
      …
      exact h,
    fails_eq := fun i => rfl
```

One field, one proof; each cites the ghosts above and the construct's readers.
`fails_eq := fun i => rfl` — the error leg is a stream-level `filterMap`, so at `Values`
the face is definitional. In Verus terms (`../DOCTRINE.md` §table): the invariant is the
loop invariant, `prove tick` is the loop body's proof, the ghosts are `assert … by`, and
each `prove` field is a `proof fn`.

## 5. A shared invariant: `OnceInv`

Two Paxos fixes (`FINDINGS.md` B1: send each ballot's P1a once; B2: recommit once per
ballot) turned out to be **the same register discipline**: a `use::state` holding the
last ballot fired at, firing only on a fresh one. It lives once, in
`Hydro/Paxos/Types.lean:750–`:

```lean
structure OnceInv {ι β : Type} {nP : Nat} (bal : ι → Ballot nP) (fired : β → Prop)
    (ro : Bool) (ra : Option (Ballot nP)) (hist : List (ι × β)) : Prop where
  reg_none : ro = true → ra = none → ∀ p ∈ hist, ¬ fired p.2
  reg_some : ro = true → ∀ r, ra = some r → ∃ p ∈ hist, fired p.2 ∧ bal p.1 = r
  reg_dom  : ro = true → ∀ r, ra = some r → ∀ p ∈ hist, fired p.2 → (bal p.1).num ≤ r.num
  once     : ro = true → List.Pairwise (fun p q => fired p.2 → fired q.2 → bal p.1 ≠ bal q.1) hist
```

with `OnceInv.init`, `OnceInv.fire`, `OnceInv.hold` as its step lemmas (`Types.lean:767–`). `hist` is `Trace.hist inputs emissions` — the zipped history, so every clause is
a membership or `Pairwise` statement and the step proofs have **no index arithmetic**
(D62/D63: the history form cut a 196-line step to 68).

`sequence_payload`'s B2 gate is then a nine-line block plus a ten-line obligation:

```lean
-- Hydro/Paxos/SequencePayload.lean:230–264 (abridged)
  tick (state recommittedAt : Option (Ballot (mem prop)) := none)
      (input view := p_relevant_p1bs)
      (input bl := H.zipTick p_ballot p_is_leader)
      (input ledLast := H.defer_tick false p_is_leader)
      (invariant ((rcGated : …) (recommittedAt : …) (view : …) (bl : …) (ledLast : …)) =>
        ∀ (me : Fin (mem prop)), <ownership of the ballot wire> → <ascent of the ballot wire> →
          OnceInv (fun x => x.2.1.1) (fun g => g ≠ 0) variant.recommitOnce
            recommittedAt (Trace.hist (Trace.zip view (Trace.zip bl ledLast)) rcGated)) :=
    let nonempty := H.bsMap (H.bcount view) (fun n => decide (n ≠ 0))
    let fire := H.bsMap (H.bsZip nonempty (H.bsZip recommittedAt (H.bsZip bl ledLast)))
      (fun (ne, (ra, ((b, l), d))) => ne && (!variant.recommitOnce || (l && !d && !decide (ra = some b))))
    rebind (recommittedAt := H.bsMap (H.bsZip fire (H.bsZip recommittedAt bl))
        (fun (f, (ra, (b, _))) => if f then some b else ra))
    yield (rcGated := H.bfilterIf view fire)
    prove init := fun _i _me _ _ => OnceInv.init,
      tick := fun i _n _out st v_t bl_t d_t hv hb hd hlen ih me hown hmono => by
        …
        simp only [rcGated_step, den]
        by_cases hfire : …
        · …; refine OnceInv.fire hx hlen … ih' … ?_; …
        · …; exact OnceInv.hold hx hlen ih' (fun h => h rfl);
```

Note the invariant's premises: the ballot wire's ownership and ascent are **inputs** to
`sequence_payload` (facts `leader_election` ensures), so the invariant quantifies over
them rather than assuming them — a face hypothesis, in the consumer's vocabulary
(`LeaderDiscipline`, chapter 03). `LeaderElection.lean`'s P1a block carries the same
`OnceInv` with its own `fired`.

## 6. What makes proofs small

The D63/D64 measurements (`FINDINGS.md`) found the mass in proofs came from four places,
each now eliminated by infrastructure rather than by effort: dependent-index reads
(`l[i]'h`, 3–8 lines of bound plumbing each) → option-indexed readers; invariants
spelled with indices → history form; a pure mirror of the body to state facts against →
the construct's `_step`/`_at`/`_reg` and `den`; faces over internal wires forcing
re-derivation downstream → faces over outputs. If a new proof feels big, it is almost
certainly re-doing one of these.
