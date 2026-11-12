---
sidebar_position: 2
title: "Tick Blocks and Their Invariants"
---

# Tick Blocks and Their Invariants

A Rust `sliced!` block with `use::state` is a loop over ticks: each iteration reads this tick's inputs and the state the previous tick left, then emits values and the next state. A fact about what such a block emits over a whole run is therefore a loop invariant. The Lean form of `sliced!` is the `tick` block, and at `Values` a block is a fold over the location's ticks, one step per tick.

[Proofs over Values](./proofs-over-values.md) used `collect_quorum`'s face from two-phase commit without looking inside it. This page proves that face's hardest field, `emit_count` (a key is emitted exactly once if it reached quorum among the responses consumed, and otherwise never), which is a statement about the state `collect_quorum` carries across arbitrarily batched ticks. You never reason about the underlying fold directly. Each block gives you **readers**, ghost facts that say what one tick does in terms of each input separately, and it accepts a loop invariant whose two obligations, the empty run and one tick, see one tick and one input at a time. The page shows what a block elaborates to, what each reader states, how to state and prove an invariant, and then the `collect_quorum` invariant from start to end.

## What a Block Elaborates To

[Tick Blocks](../programs/ticks-and-cycles.mdx#tick-blocks) shows how to write a block. The elaborator turns the block into ordinary `let`s, and these are the names your proofs use:

- `<out₁>_step`, named after the first output: the per-tick step `fun me st inpt => …`, from the states and this tick's inputs to the next states and the emissions. It is a plain `let`, and at `Values` every in-tick operator has a `den` rule, so `simp only [<out₁>_step, den]` turns one tick into the plain `List`/`Multiset` term of the Rust lines.
- `<out₁>_r := H.tick_scan … (x₁, …) <out₁>_step seeds`, the fold, and one wire per emission, projected from it. Each such emitted wire is an **output** of the block.
- The readers below, as `ghost have`s.

Do not unfold `tick_scan`: every fact you need about it is a reader, stated over the plain fold below.

## The Readers

At `Values` the fold is `scanAcrossTicksTrace` (`Hydro/Trace.lean`), a Mealy-machine run over the zipped input trace:

```lean
def scanAcrossTicksTrace {ι σ β : Type _} (g : σ → ι → σ × β) : σ → List ι → List β
  | _, [] => []
  | s, x :: xs => (g s x).2 :: scanAcrossTicksTrace g (g s x).1 xs
```

and `scanAcrossTicksState g seed l` is the register after consuming `l`. Stated directly over this fold, a fact about tick `n` would be about the `n`-th element of a zip of every input, a nested tuple you would have to take apart in every proof. The readers restate the fold per input and per tick instead. Every block gets readers for each output `out`, in scope after the block like any `ghost have`. In the table, `Z i` is the zipped input trace `Trace.zip (x₁ i) (… (xₘ i))`, and `S n` is `scanAcrossTicksState (<out>_step i) seed ((Z i).take n)`, the register before tick `n`; `proj` projects output `out` from the step's emissions. The last two readers exist only for a block that declares an invariant `P` ([below](#the-invariant-normal-form)); `spectators` are the extra names `P` mentions.

| reader | statement | emitted for |
|---|---|---|
| `h<out>_run` | `∀ i, out i = scanAcrossTicksTrace (<out>_step i) seed (Z i)`, projected to output `out` for a multi-output block | every block |
| `h<out>_at` | `∀ i n e, (out i)[n]? = some e ↔ ∃ x₁_t …, (x₁ i)[n]? = some x₁_t ∧ … ∧ e = proj (<out>_step i (S n) (x₁_t, …)).2` | every block |
| `h<out₁>_reg` | `∃ reg,` the seed law ∧ per output the `_at` clause at `reg i n` ∧ the step law ∧ one stall law per input [∧ the invariant before every tick] | blocks with state, named after the first output |
| `h<out>_inv` | `∀ i, P (out i) (final states) spectators` | blocks with an `invariant` |
| `h<out>_inv_take` | `∀ i n, P ((out i).take n) (S n) spectators` | blocks with an `invariant` |

**`_at` is indexed by `Option` and split per input.** Its right-hand side gives one read `(xⱼ i)[n]? = some xⱼ_t` per input, never a read of the zip. You `obtain` exactly the inputs you care about and discard the rest with `-`. An invariant's obligations take the same shape (below).

**`_reg` hides the fold.** The register is existentially quantified: a `reg : Fin _ → Nat → σ` with its seed, read, step and stall laws. After `ghost obtain ⟨reg, …⟩` you reason about `reg i n` as an opaque sequence and never see `scanAcrossTicksState`. A stall law, `(xⱼ i)[n]? = none → reg i (n+1) = reg i n`, says the register freezes once any input has ended, because the zip stops at the shortest input.

**`_run` is for whole-run facts.** Use it where a statement is about the entire output, such as its length (`scanAcrossTicksTrace_length`); use `_at` for one tick.

In use, a reader turns a fact about tick `n` of the output into a fact about one step on that tick's inputs, and `simp only [<out>_step, den]` evaluates that step as plain `List`/`Multiset` operations. [Consuming the Readers](#consuming-the-readers) does this for `collect_quorum`.

## The Invariant Normal Form

An invariant is a predicate `P` over the output emitted so far, the registers, and any other names it needs. You declare it in the block's header and prove its two obligations in a `prove` clause after the emissions (the grammar from `Hydro/HydroTick.lean`):

```lean
    tick (state r₁ : τ₁ := seed₁) … (state sₖ : H.BoundedStream α ord ret) …
      (input x₁ := <wire₁>) … (input xₘ := <wireₘ>)
      (invariant (<out> <r₁> … <rₙ> <spectators…>) => P)? :=
      <per-tick let-chain over r₁ … rₙ x₁ … xₘ (this tick's values)>
      rebind (r₁ := e₁, …, rₙ := eₙ)
      emit (a := …, …)? yield (b := …, …)? kyield (c := …, …)?
      (prove init := <P at [] seeds>, tick := <one-tick step>)?;
```

The binders are positional. The first is the output emitted so far, which must be typed `(out : List τ)`, because at `Values` a step's components are `BoundedSingleton _`, which is only definitionally `τ`. Then come every state, in declaration order. Any remaining names are **spectators**: names in scope (usually the input wires) that `P` receives at member `i`. The two obligations are:

- `init`: `∀ i, P [] seeds`. A stream state's seed is the empty pool of its grade, written `ValuesTick.seed …`.
- `tick`: `∀ i n out st x₁_t …, (x₁ i)[n]? = some x₁_t → … → out.length = n → P out st → P (out ++ [(step st (x₁_t, …)).2]) (step st (x₁_t, …)).1`.

The hypothesis `out.length = n` ties the number of emissions so far to the tick index. With it, `P` can talk about the consumed input prefix `xⱼ.take out.length`. Most real invariants relate the registers to this prefix: "the registers summarize the input consumed so far". The underlying theorem is a loop induction over the zipped trace (`Hydro/Trace.lean`):

```lean
theorem scanAcrossTicks_invariant? (g : σ → ι → σ × β)
    (P : List β → σ → Prop) (l : List ι) (seed : σ)
    (hinit : P [] seed)
    (htick : ∀ (n : Nat) (out : List β) (st : σ) (x : ι),
      l[n]? = some x → out.length = n → P out st →
      P (out ++ [(g st x).2]) (g st x).1) :
    P (scanAcrossTicksTrace g seed l) (scanAcrossTicksState g seed l) :=
```

The clause splits the zipped read `l[n]? = some x` into one read per input before handing the obligation to you, so your `tick` proof never mentions the tuple. From the two obligations the elaborator produces `h<out>_inv` (the whole run, final state), `h<out>_inv_take` (every prefix, with the state before that tick), and the invariant conjunct of `_reg`.

The clause is available on single-output blocks. The elaborator rejects a missing `prove` (``tick: an `invariant` clause needs its trailing `prove init := …, tick := …` ``) and an untyped output binder (``tick invariant: type the output binder as `(out : List τ)` ``).

A block's invariant covers the state the block carries from tick to tick. State carried around a `forward_ref` cycle, through a network hop or a `defer_tick`, is a different construct with its own invariant clause ([Knots and Loop Invariants](./knot-invariants.md)).

## The `collect_quorum` Invariant

`collect_quorum` emits each key once, in the tick where the key reaches `min` successful responses. It keeps responses that are still below quorum in the **window** `not_all` across arbitrarily batched ticks. The **consumed** responses are those the batch decision has delivered so far (`cqConsumed`). The contract field that is hard to prove is `emit_count`. Under the **usage contract** (`1 ≤ min ≤ max`, and no key receives more than `max` responses), a key is emitted exactly once if it reached `min` among the consumed responses, and otherwise not at all. Whether a key has been emitted depends on what the registers held at every earlier tick, so the field needs an invariant.

### The Invariant

The invariant says that both registers are determined, key by key, by the consumed prefix `pfx`, the sum of the batches consumed so far (`Hydro/Std/Quorum.lean`):

```lean
structure CQRegInv (min max : Nat) (notAll : Multiset (K × Except E V))
    (mbnm : Multiset K) (pfx : Multiset (K × Except E V)) : Prop where
  /-- The window never exceeds the consumed responses. -/
  window_le : notAll ≤ pfx
  /-- Under the cap, the window's key part is the consumed key part until
  the drop point, then empty. -/
  window : 1 ≤ min → min ≤ max → ∀ k, cqKeyCount pfx k ≤ max →
    notAll.filter (fun r => r.1 = k)
      = if cqDropped min max pfx k then 0 else pfx.filter (fun r => r.1 = k)
  /-- Under the cap, `min_but_not_max` names the keys at `min` `Ok`s below
  `max` responses. The register is only read when `min < max`. -/
  locked : 1 ≤ min → min < max → ∀ k, cqKeyCount pfx k ≤ max →
    (k ∈ mbnm ↔ (min ≤ cqOkCount pfx k ∧ cqKeyCount pfx k < max))
```

The **drop point** `cqDropped` marks the moment the Rust removes a key from the window. That is reaching `min` `Ok`s when `min = max`, and `received_from_all` (`max` responses of either kind) otherwise. The cap `cqKeyCount pfx k ≤ max` in `window` and `locked` comes from the Rust code's behaviour after a drop. Any further response for a dropped key starts a fresh window, so the key could reach `min` again and be emitted twice. quorum.rs assumes this cannot happen, because a quorum of `max` participants answers each key at most once. Under the cap, `dropped_kills_batch` proves that a dropped key receives nothing more:

```lean
theorem dropped_kills_batch {pfx b : Multiset (K × Except E V)} {k : K}
    (hcap : cqKeyCount (pfx + b) k ≤ max)
    (hd : cqDropped min max pfx k) :
    b.filter (fun r => r.1 = k) = 0 := by
```

The soundness field `emit_sound` therefore needs no cap, but `emit_count` assumes `1 ≤ min ≤ max` and the cap (see [Contracts](../programs/contracts.md)).

The two `rebind` lines are the same in `collect_quorum` and `collect_quorum_with_response` in `quorum.rs`. The loop-body proof is therefore stated once, as two lemmas about those formulas, one per Rust branch:

```lean
theorem step_eq {notAll pfx b : Multiset (K × Except E V)} {mbnm : Multiset K}
    (hmm : min = max) (ih : CQRegInv min max notAll mbnm pfx) :
    CQRegInv min max
      ((notAll + b).filter (fun r => ¬ min ≤ cqOkCount (notAll + b) r.1))
      mbnm (pfx + b) := by
```

`step_ne` is the same for `min ≠ max`, where the window drops keys heard from everyone and `min_but_not_max` is recomputed.

### The Block

The invariant names the output, both states, and the input wire as a spectator. Its consumed prefix is `new_inputs.take just_reached_quorum.length`, and the hypothesis `out.length = n` turns this length into the tick index:

```lean
      (invariant ((just_reached_quorum : List (Multiset K))
          (not_all : Multiset (K × Except E Unit)) (min_but_not_max : Multiset K)
          (new_inputs : Trace (Multiset (K × Except E Unit)))) =>
        CQRegInv min max not_all min_but_not_max
          ((new_inputs.take just_reached_quorum.length).sum)) :=
```

The obligations:

```lean
    -- init: before any tick, both registers are empty and nothing is consumed
    prove init := fun _i => by
        simp only [List.length_nil, List.take_zero, List.sum_nil, ValuesTick.seed_pair,
          ValuesTick.seed_stream, den]
        exact CQRegInv.init,
      -- tick: if the invariant holds before tick `n`, whose batch is `b_t`, it
      -- holds after it
      tick := fun i n out st b_t hb hlen ih => by
        -- the consumed prefix after the tick is the one before it plus `b_t`
        simp only [List.append_eq, List.length_append, List.length_singleton, hlen,
          List.take_add_one, hb, Option.toList_some, List.sum_append, List.sum_singleton] at ih ⊢
        -- evaluate one tick of the block as multiset operations
        simp only [just_reached_quorum_step, den]
        -- the Rust's two branches, `max == min` and otherwise
        by_cases hmm : max = min
        · simp only [if_pos hmm, cq_reached_eq, Bool.not_eq_true', decide_eq_false_iff_not,
            filter_notmem_keys_filter]
          exact CQRegInv.step_eq hmm.symm ih
        · simp only [if_neg hmm, cq_reached_eq, cq_keys_filter_eq, decide_eq_true_eq,
            cqOkCount_add_cqErrCount, Bool.not_eq_true', decide_eq_false_iff_not,
            filter_notmem_keys_filter, keys_filter_notmem_keys_filter]
          exact CQRegInv.step_ne (Ne.symm hmm) ih;
```

What each step does in `tick`:

1. The binders are `i n out st`, then one read `b_t` with its hypothesis `hb : (new_inputs i)[n]? = some b_t` (the block has one input), then `hlen : out.length = n` and `ih`.
2. The first `simp only` rewrites the consumed prefix. With `hlen` and `List.take_add_one` plus `hb`, `((new_inputs i).take (out ++ [_]).length).sum` becomes `((new_inputs i).take n).sum + b_t`, the `pfx + b` shape of `step_eq`'s conclusion.
3. `simp only [just_reached_quorum_step, den]` evaluates the generated step. `bchain` becomes `+`, `bantiJoin` and `bfilterNotIn` become filters on the complement, and the keyed fold `bkfold cqCount` becomes the counter `cqCounter` of the window.
4. Inside the Rust `if max == min` case split, `cq_reached_eq` rewrites `.entries().filter_map(… if success >= min …)` on the counter to `(pool.map Prod.fst).dedup.filter (fun k => min ≤ cqOkCount pool k)`, and `cq_keys_filter_eq`, `filter_notmem_keys_filter` and `keys_filter_notmem_keys_filter` do the same for `received_from_all` and the two anti-joins. The goal is then `step_eq`'s or `step_ne`'s conclusion.

### Consuming the Readers

After the block, the contract proof reads one tick through `_at` and the state before it through `_inv_take`:

```lean
  ghost have htick : ∀ (i : Fin (mem ℓ)) (n : Nat) (e : Multiset K),
      (just_reached_quorum i)[n]? = some e →
      ∃ (b_t : Multiset (K × Except E Unit)) (S : Multiset (K × Except E Unit) × Multiset K),
        (new_inputs i)[n]? = some b_t
        ∧ CQRegInv min max S.1 S.2 ((new_inputs i).take n).sum
        ∧ e = (if max = min then
            ((S.1 + b_t).map Prod.fst).dedup.filter (fun k => min ≤ cqOkCount (S.1 + b_t) k)
          else
            (((S.1 + b_t).map Prod.fst).dedup.filter (fun k => min ≤ cqOkCount (S.1 + b_t) k)).filter
              (fun k => k ∉ S.2)) := fun i n e he => by
    -- tick `n` exists
    have hn : n < (just_reached_quorum i).length := Trace.read_lt he
    -- its emission `e` is one step of the block on that tick's batch `b_t`
    obtain ⟨b_t, hb, rfl⟩ := (hjust_reached_quorum_at i n e).mp he
    -- and the registers before it satisfy the invariant
    have h := hjust_reached_quorum_inv_take i n
    simp only [List.length_take_of_le (Nat.le_of_lt hn)] at h
    refine ⟨b_t, _, hb, h, ?_⟩
    simp only [just_reached_quorum_step, den]
    by_cases hmm : max = min
    · simp only [if_pos hmm, cq_reached_eq]
    · simp only [if_neg hmm, cq_reached_eq, Bool.not_eq_true', decide_eq_false_iff_not]
```

`_inv_take i n` states the invariant at `(out.take n).length`. `List.length_take_of_le` turns that into `n` once `Trace.read_lt` has shown that tick `n` exists. Two more ghosts finish the argument. `hcount_tick` says a single tick emits `k` exactly when `k` crosses `min` on that tick, from `htick`, `CQRegInv.window_kpart` and `dropped_kills_batch`. `hcount` sums those per-tick indicators by induction on `n`. The `prove` clause then closes `emit_count` over the whole run with `_run`:

```lean
    emit_count := fun i k h1 hmx hcap => by
      show ((just_reached_quorum i).sum).count k = _
      have hlen : (just_reached_quorum i).length = (new_inputs i).length := by
        rw [hjust_reached_quorum_run i, scanAcrossTicksTrace_length]
      …
```
