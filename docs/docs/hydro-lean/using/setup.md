---
sidebar_position: 3
---

# Setup

Hydro Lean is the Lean 4 project in `hydro_lean/`. In Hydro Lean the build runs the test suite: every smoke test, every decision census, every generated theorem and every axiom report runs while the library compiles. Run every command on this page from `hydro_lean/`.

## Toolchain

`hydro_lean/lean-toolchain` pins `leanprover/lean4:v4.33.1`, and `lakefile.toml` requires Mathlib at the matching tag `v4.33.1`. You install the toolchain with `elan` and download Mathlib's prebuilt files with `lake exe cache get`:

```bash
curl -sSf https://elan.lean-lang.org/elan-init.sh | sh -s -- -y --no-modify-path --default-toolchain none
export PATH="$HOME/.elan/bin:$PATH"
elan toolchain install "$(cat lean-toolchain)"
lake exe cache get        # ends with "Decompressed N file(s)" or "Already decompressed N file(s)"
```

## Build

```bash
lake build Hydro Examples
```

The build takes several minutes once Mathlib comes from the cache and finishes with `Build completed successfully`. A green build means that every `#guard` and `#guard_eval` passed, every `#nondet_census` found the decision counts written on its line ([Nondeterminism](./programs/nondeterminism.md#the-census)), every `hydro def` generated and proved its artifacts ([Generated Artifacts](../internals/generator/generated-artifacts.md)), and `Hydro/AxCheck.lean` and `Examples/AxCheck.lean` printed the axioms of each theorem they audit:

```text
info: Examples/AxCheck.lean:93:0: 'Hydro.Tutorials.vote_quorum_safe_wired' depends on axioms: [propext, Classical.choice, Quot.sound]
```

The [Quickstart](./quickstart/) runs a program as processes with `lake exe quorum`, which builds the executable on first use.

## Working on One File

After a full build, you can elaborate one file against the built dependencies:

```bash
lake env lean Examples/Tutorials/HelloWorld.lean
```

To see a generated artifact's statement, write a probe file inside `hydro_lean/` and elaborate it the same way:

```lean
import Examples
open Hydro

#check @Tutorials.hello_broadcast_co_rr₁
#print axioms Tutorials.hello_safe_wired
```
