# The guided tour

Read in order. You know Hydro (the Rust library) and some Lean 4; you have never seen
this tree. Each chapter is short, quotes the tree verbatim with a `file:line` anchor so
you can re-check it, and links into the reference docs at the root
(`../ARCHITECTURE.md`, `../DOCTRINE.md`, `../CORRESPONDENCE.md`, …) for depth.

| chapter | question it answers |
|---|---|
| [01 From Rust to Lean](01-from-rust-to-lean.md) | what does a Hydro function look like as a `hydro def`? (`collect_quorum`, side by side with `quorum.rs`) |
| [02 What a program denotes](02-what-a-program-denotes.md) | what *is* the Lean program's value — traces, pools, decisions as inputs; reading a wire with `den`; why running it is the denotation |
| [03 Contracts](03-contracts.md) | `ensures` faces over outputs, and how a consumer (`two_pc`) composes by contract alone |
| [04 Ghosts and proofs](04-ghosts-and-proofs.md) | where proofs live in a program: `ghost have`, `prove`, loop invariants, the construct's readers — `OnceInv` end to end |
| [05 Knots](05-knots.md) | `fix … complete`, the generated Kleene chain, and K4 — `paxos_core`'s invariant read line by line |
| [06 The machine and the corner](06-the-machine-and-the-corner.md) | `SchedSem`, pacing, cursors; what `paxos_safe_sched'` actually says and why it has no premise |
| [07 Liveness as chains](07-liveness-chains.md) | `cq_live_chain`: fairness in decision vocabulary |
| [08 Adding a module](08-adding-a-module.md) | the checklist, start to gate |
| [09 Extending the engine](09-extending-the-engine.md) | when a new operator is justified and what it costs |

Line numbers are as of D68 and may drift; declaration names are the stable anchors.
