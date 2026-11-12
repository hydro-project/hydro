---
sidebar_position: 3
title: "Knots and Loop Invariants"
---

# Knots and Loop Invariants

A **knot** is a cycle in the dataflow, Rust's `forward_ref`, written as a `fix … complete` block in Lean ([Cycles](../programs/ticks-and-cycles.mdx#cycles) shows how to write one). Paxos has two at the top level: the acceptor log `a_log` and the proposers' max-ballot stream. Slot agreement is a fact about what flows around these cycles, so proving it means reasoning about a fixpoint.

At `Values` a knot is the **fuel**-th Kleene iterate of its body, starting from the empty collection. The fuel is a decision of type `H.FixDec`: a `Nat` at `Values`, and `Unit` at the step machine `SchedSem`, where a knot's wire at each step is its body's output at that step and needs no iteration count. Every theorem holds for every fuel, so the closed wire at `Values` is not a true fixpoint: applying the body to it gives the next iterate, which is at least as large and in general larger. Three rules for proofs follow from this:

- A fact about a knot is an induction over its **stages**, the iterates `0, 1, …, fuel`.
- A fact that relates the body's input to its output is a prefix or sub-pool relation (`⊑`), never an equation.
- A fact the body needs about its own loop input has to be shown at every stage.

The tick invariants of [the previous page](./tick-invariants.md) cover state a block carries from one tick to the next at one location. A knot carries state around the program, usually through the network, and its invariant is a separate clause on the `fix` block. This page shows the facts you can state inside the block, the stage vocabulary the generator provides, the `invariant` clause and its assumable form, and then the Paxos induction on `a_log`.

## Facts Inside a Cycle

A `ghost have` inside the `fix` body is stated over the loop wires, and the `prove` clause receives it at the closed wires. Because a face holds for every input, the place to state a callee's face is inside the cycle, where the callee is applied; the contract proof then has it at the closed knot. `paxos_core` does this for both of its callees:

```lean
      -- the sequencing contract at these wires (replayed likewise)
      ghost have hsp := sequence_payload.ensures variant prop acc c_to_proposers
        a_checkpoint le.1 le.2.1 le.2.2.1 config.f le.2.2.2 dec.sp sched.sp
```

## The Stage Vocabulary

An induction over stages needs names for them. For each knot `K` the generator emits its Kleene chain at `Values`, `K.stages … m`, the `m`-th iterate from the empty wire, and four laws (for `toy_loop.w`, shortened to their conclusions):

```lean
toy_loop.w.stages ℓ inp tdec sched 0 = MemberMap.mk' fun i => PoolBot StrOrd.noOrder Retries.exactlyOnce
toy_loop.w.stages ℓ inp tdec sched (m + 1) =
    (toy_loop.body (Values L mem) ℓ inp tdec sched (toy_loop.w.stages ℓ inp tdec sched m)).1
toy_loop.w (Values L mem) ℓ inp tdec sched = toy_loop.w.stages ℓ inp tdec sched tdec.fuel
m ≤ m' → ∀ (i : Fin (mem ℓ)),
      PoolLe StrOrd.noOrder Retries.exactlyOnce (MemberMap.get (toy_loop.w.stages ℓ inp tdec sched m) i)
        (MemberMap.get (toy_loop.w.stages ℓ inp tdec sched m') i)
```

(`stages_zero`, `stages_succ`, `stages_fix`, `stages_mono`). `toy_loop.body` is the `fix` block's body as a function of the loop wire. `stages_mono` makes the stages an ascending chain. Its order, `PoolLe` per grade for streams and list prefix `<+:` for tick wires, is the `⊑` used below.

## The Invariant Clause

You rarely cite `stages` directly. The `invariant` clause packages the induction over them, so that you prove one base case and one step:

```lean
    fix (w : τ) via f
      invariant w (le sp le' sp') => P
        , base := proof
        , step := proof
      := body
```

A block that closes several wires takes the clause only on its first wire, and only in the single-source form with `complete`. The optional list after `w` names chain `let`s of the body. An unprimed name (`le`) is that `let`'s value at the current stage, and a primed name (`le'`) is its value at the closed knot. `P` may also mention `w'`, the closed knot. Each obligation is stated over opaque binders that carry those source names, so your proof never re-spells a module application. In addition, for every listed `let` whose value is a module call with a face, the obligation receives that face at each valuation it mentions. With `C.ensures`, the face of `le := C …` arrives as `hle` (current stage), `hle_step` (next stage) and `hle'` (closed knot). Intro order is: binders, then premises in the order the table lists them, then `P`'s own antecedents.

| | binders | premises | goal |
|---|---|---|---|
| `base` | `w'`, lets at the seed, primed lets | `htop`, faces at the top (`h<let>'`), faces at the seed (`h<let>`) | `P` at the seed |
| `step` | `w'`, lets at `w`, lets at the next stage (`<let>_step`), primed lets | `htop`, `h<let>'`, `h<let>`, `h<let>_step`, then `w ⊑ w'`, `body w ⊑ w'`, `w ⊑ body w`, the induction hypothesis | `P` at `body w` |

`htop : w' ⊑ body w'` says the chain top is below its own image (`stages_mono` at `fuel ≤ fuel + 1`). With the `⊑` premises it lets a step move facts from the current stage up to the closed knot. The underlying theorem is a bounded induction (`Hydro/Trace.lean`):

```lean
theorem iterate_invariant {τ : Type _} {F : τ → τ} {x : τ}
    {I : τ → Prop} :
    ∀ (n : Nat), I x →
      (∀ m, m < n → I (iterate F x m) → I (F (iterate F x m))) →
      I (iterate F x n)
```

The step's chain premises refer to the fuel, so the induction is cut at the fuel. The clause produces two theorems: `<M>.<w>.inv`, `P` at the closed knot, which the `prove` clause receives as `h<w>_inv`; and `<M>.<w>.inv_stages`, `P` at every stage up to the fuel. `toy_loop`'s clause is the smallest example:

```lean
  fix (w : H.Stream ℓ Nat .noOrder .exactlyOnce) via tdec.fuel
    -- the invariant: the loop's relay output at each stage (`out`) is below
    -- its output at the closed loop (`out'`); both obligations follow from
    -- `toy_step`'s monotonicity
    invariant w (out out') => ∀ i, out i ≤ out' i,
      base := fun _htop => toy_step_mono₁ (inp := inp) (inp' := inp)
        ℓ tdec.relay sched
        (fun _ => PoolLe.bot_le _ _ _) (fun _ => PoolLe.refl _ _ _),
      step := fun _htop _hw hbw _hbw0 _ih => toy_step_mono₁ (inp := inp)
        (inp' := inp) ℓ tdec.relay sched hbw
        (fun _ => PoolLe.refl _ _ _)
```

In term mode the `let` binders need no names, because Lean reduces them away when it elaborates `fun _htop => …`. In tactic mode `intro` names them, as in the Paxos proof below. The generated stage form carries the fuel bound:

```lean
@toy_loop.w.inv_stages : ∀ {L : Type} {mem : L → ℕ} (ℓ : L)
  (inp : HydroSem.Stream mem ℓ ℕ StrOrd.noOrder Retries.exactlyOnce) (tdec : ToyLoopDec (Values L mem) (mem ℓ))
  (sched : ToyStepSched (Values L mem) (mem ℓ)),
  ∀ m ≤ tdec.fuel,
    …
```

A wrong name inside an obligation does not fail at the site. Lean's error recovery turns it into a term with dead free variables, which only the asynchronous kernel would reject, far from the cause. The clause checks for this itself and reports `fix invariant: residual free variables in the packaged induction … (an obligation proof likely references an unknown name)`.

## Assumable Invariants

Sometimes the body needs a fact about its loop input. The case in the repository is a conditional fold inside the body (a fold whose commutativity holds only under a pairwise prerequisite, discharged by `ghost pre`; see [Proofs over Values](./proofs-over-values.md#requires-and-ghost-pre)) whose prerequisite depends on recirculated content. No `ghost pre` can prove it from the body alone, because the wire's content is the knot's own output. When `P` mentions only `w`, so neither a listed `let` nor `w'`, the body may **assume** it of its input, as a loop body assumes its loop invariant on entry (`toy_pre_knot`, `Hydro/HydroTickCheck.lean`):

```lean
  fix (w : H.Stream ℓ (Nat × (Nat × Nat)) .noOrder .exactlyOnce) via tdec.fuel
    -- the invariant, assumed of the loop input: every element's third field
    -- is twice its second
    invariant w (out) => (∀ i, ∀ a ∈ w i, a.2.2 = a.2.1 * 2),
      -- the empty wire has no elements
      base := by
        intro _w' _out i a ha
        exact absurd ha (Multiset.notMem_zero a),
      -- one iteration: every output element is a mapped input, `(k, (b, b * 2))`
      step := by
        intro _w' out _out_step _ih i a ha
        simp only [out, den] at ha
        obtain ⟨a', -, rfl⟩ := Multiset.mem_map.mp ha
        rfl :=
    let inp := H.map src (fun _me kb => (kb.1, (kb.2, kb.2 * 2)))
    -- the loop closes through a hop, so the body is strict in `w`
    let fed := H.union inp (H.values (H.broadcast_closed sched.net w))
    let keyed := H.tinto_keyed (H.batch fed tdec.batch)
    …
    ghost pre hpre : ∀ i, (KeyedPool.accum (keyed i)).Pairwise ToyTie := by
      …
        exact hw_inv j c hcm
```

The test is syntactic and runs before the body elaborates. When it passes, three things change for you:

- **The body may use `hw_inv`.** Inside the body's `ghost pre` proofs the invariant of the loop input is in scope as `h<w>_inv` (here `hw_inv`), next to `hreq` when the module has its own `requires`. Inside the body `w` is a stage, not the closed knot, so `hw_inv` says the invariant holds of whatever stage the body is run at.
- **The step may not use the fuel.** The transfer lemmas run the body at arbitrary stages, so the invariant must hold at every stage, not only up to the fuel, and the induction is the unbounded `iterate_invariant_all`. Its step may use nothing that refers to the fuel: `htop` and the `⊑` premises are not offered. The step's binders are `_w' out _out_step _ih`, and the stage theorem has no bound:

```lean
@toy_pre_knot.w.inv_stages : … (m : ℕ) (i : Fin (mem ℓ)),
  ∀ a ∈ MemberMap.get (toy_pre_knot.w.stages ℓ src wm tdec sched m) i, a.2.2 = a.2.1 * 2
```

- **Nothing is left to discharge.** The `base` and `step` proofs are the whole obligation; you do not prove the assumption anywhere else.

A predicate over chain `let`s describes the body's output, not its input, so it can only be exported, never assumed.

## The Paxos Induction on `a_log`

Safety for `paxos_core` is `SlotFunctional`: two commits at one slot carry the same value. The protocol argument is quorum intersection plus a provenance regress. A value opened at a higher ballot traces back, through the logs the leader read, to a value opened earlier. The clause on the innermost wire states this regress:

```lean
    invariant a_log (le sp le' sp') =>
      variant = .guarded → mem acc ≤ 2 * config.f + 1 →
      ∀ (slot : Nat) (b₁ b₂ : Ballot (mem prop)) (val₁ v₂ : Option P)
        (i₂ : Fin (mem prop)),
        SPChosen config.f (fun j => (a_checkpoint j).val (dec.sp.ap2.ckSnap j))
          le'.2.2.2 sp'.2.1 slot b₁ →
        (∀ (j : Fin (mem acc)) {t : Nat} {lg : ALog P (mem prop)}
          {e : LogValue P (mem prop)},
          (sp'.2.1 j)[t]? = some lg → (slot, e) ∈ lg.2 → e.ballot = b₁ →
          e.value = val₁) →
        b₁.blt b₂ = true →
        LeaderOpen (le.1 i₂) (le.2.1 i₂) (le.2.2.1 i₂) b₂ slot v₂ →
        v₂ = val₁,
```

Two predicates carry the Paxos content. `SPChosen f … slot b₁` says `f + 1` distinct acceptors voted for `(slot, b₁)`: each one's max-ballot input held `b₁` at a tick whose published log already covered the slot at that ballot (write-before-ack). `LeaderOpen … b₂ slot v₂` says that at some leader tick with ballot `b₂`, the promise view the leader read determines `v₂` as the value it proposes for `slot` (it **opens** `v₂`). The invariant then reads: suppose `val₁` is chosen at ballot `b₁` at the closed knot (primed runs), and the entries at `(slot, b₁)` agree on it. Then any value that **this stage's** leader opens at a higher ballot `b₂` (unprimed runs) is `val₁`. The predicate mentions the chain lets, so it is not assumable. The obligations use the faces `hle`/`hsp` of the two callees at each valuation, plus the chain orders.

**Base.** At the seed the log is empty. A leader tick needs a promise quorum, and each promise quotes an `a_log` tick, of which there are none:

```lean
      base := (by
        -- the seed stage opens nothing: no acceptor has ever ticked, so
        -- a leader tick's promise quorum cannot exist
        intro a_log' le sp le' sp'
        intro _htop _hle' _hsp' hle _hsp _hvar _hnA
        intro slot b₁ b₂ val₁ v₂ i₂ _hch _hval _hlt hopen
        exfalso
        obtain ⟨t, view, hl, -, -, -, -⟩ := hopen
        obtain ⟨hpl, hl⟩ := List.getElem?_eq_some_iff.mp hl
        obtain ⟨hpr2, _, hcard, hprom⟩ := hle.view_promise i₂ hpl hl
        obtain ⟨v0, hv0⟩ : ∃ v0, v0 ∈ (le.2.2.1 i₂)[t]'hpr2 :=
          Multiset.card_pos_iff_exists_mem.mp (by omega)
        obtain ⟨j, tj, htj, -⟩ := hprom v0 hv0
        simp at htj),
```

**Step.** The intro lines follow the obligation's layout in the table above:

```lean
      step := (by
        intro a_log' le sp le_step sp_step le' sp'
        intro htop hle' hsp' hle hsp hle_step hsp_step hw hbw _hbw0 ih
        intro hvar hnA
        intro slot b₁ b₂ val₁ v₂ i₂ hch hval hlt hopen
        -- this stage's published log is below the top's (the
        -- write-before-ack step, then the top's own)
        have hpublift : ∀ jj, sp.2.1 jj <+: sp'.2.1 jj :=
          fun jj => (hbw jj).trans (htop jj)
```

`hpublift` applies the chain-order idiom. `hbw` places this stage's closing value (`sp.2.1`, the log that sequencing writes) below the closed knot, and `htop` places the closed knot below its own image `sp'.2.1`. Every fact read off this stage's log can therefore be lifted to the top. The rest of the step:

1. `hle_step.providers` gives the opening leader `f + 1` distinct promisers. The chosen hypothesis gives `f + 1` distinct voters. With `mem acc ≤ 2f + 1`, `nodup_inter_of_length` (pigeonhole) gives a common acceptor `j`.
2. On `j`'s max-ballot wire, which only ascends (`hle'.max_mono`), the vote at `b₁` comes before the promise at `b₂`. So the log `j` promised from covers `(slot, b₁)` (`hsp'.log_covers_mono`), or the slot was garbage-collected below the checkpoint, which the leader never opens.
3. The opened value is the champion of the leader's view at that slot, so its ballot is at least `b₁`. If the ballot equals `b₁`, the champion's entry, lifted by `hpublift`, carries `val₁`. If it is strictly higher, the champion is a published entry of this stage's log, so this stage's leader opened it (`hsp.log_entry_open`), and the induction hypothesis applies:

```lean
          have ho := hsp.log_entry_open hreq hvar j₀ hlg₀ hen₀
          exact ih hvar hnA slot b₁ best.ballot val₁ best.value
            best.ballot.proposerId hch hval hblt ho)
```

`ih` is `P` at the current stage, and `hopen` is a `LeaderOpen` at the next stage (`le_step`). The **Kleene stage is the induction measure** of the provenance regress. A value opened at stage `m + 1` either is the chosen value or comes from a value opened at stage `m`. No induction over ballots or time appears anywhere. The `prove` clause uses the closed form, where `le = le'`:

```lean
      slot_functional := fun hvar hnA => by
        subst hvar
        -- the packaged knot induction, at the closed wires
        have hI := ha_log_inv rfl hnA
        …
        obtain ⟨b, b', hbb, W, W'⟩ := hsp.commit_distinct hreq' rfl h h' hne
        cases hc : b.blt b' with
        | true => exact hne (hI slot b b' v v' i' W.chosen W.entries_agree hc W'.opened).symm
```

## Kleene Ascent Through `MonoRel`

`leader_election`'s flag knot needs no invariant. It needs the second rule listed at the top of this page, a prefix relation between the body's input and output. `p_p1b`'s face `ballot_stable` has the requirement `solicited_at_follower`: every `Ok` reply pins a tick at which the flag the body writes (`pp.1`) was false. The heartbeat's trigger gate shows only that the flag the body reads (the loop input `p_is_leader`) was false. At a true fixpoint the two flags would be equal. At finite fuel the output is the next iterate, so what holds is that the input is a prefix of the output, by Kleene ascent:

```lean
    -- the flag the body reads is a prefix of the flag it writes, at every member
    ghost have hknot : ∀ i, p_is_leader i <+: pp.1 i :=
      iterate_le_succ
        (R := fun (a b : TickV (mem prop) Bool) =>
          ∀ i, a i <+: b i)
        (F := fun f => (pass (fails (iam f) f) (iam f) f).2.2.1)
        (x := .mk' fun _ => [])
        (fun _ => List.nil_prefix)
        (fun {a b} hab => hstep hab)
        dec.fuelLead
```

`iterate_le_succ` needs the body to preserve `⊑` (`hstep`). The body is `leader_election.body`, the `fix` block's body as a function of the loop wires, named with `ghost let pass`. Its monotonicity needs no separate proof: instantiating it at `MonoRel L mem` produces a pair of related runs, and `.property` proves the relation. The two inner knots `fails`/`iam` are coupled under the two outer flags by `iterate_mono_param`. The ghost opens callee bodies with `with_unfolding_all`, because callee modules are opaque inside ghosts otherwise:

```lean
    ghost have hstep : ∀ {a b : TickV (mem prop) Bool},
        (∀ i, a i <+: b i) →
        ∀ i, (pass (fails (iam a) a) (iam a) a).2.2.1 i
          <+: (pass (fails (iam b) b) (iam b) b).2.2.1 i := by
      -- the `MonoRel` instantiation reads through the callees' bodies:
      -- callee modules are opaque to a ghost's elaboration otherwise
      with_unfolding_all
        intro a b hab
        …
```

The consumer moves a read from the cycle input to the output flag, where `ballot_stable` needs it:

```lean
        have hpre : p_is_leader r <+: pp.1 r := hknot r
        have hu_pp : u < (pp.1 r).length :=
          Nat.lt_of_lt_of_le hf hpre.length_le
        refine ⟨u, hbu, ?_⟩
        rw [List.getElem?_eq_getElem hu_pp, ← List.IsPrefix.getElem hpre hf, hff]
```
