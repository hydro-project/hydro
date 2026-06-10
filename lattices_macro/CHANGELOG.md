

## v0.6.1-alpha.0 (2026-06-10)

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 1 commit contributed to the release.
 - 40 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release hydro_build_utils v0.1.1-alpha.0, dfir_lang v1.0.0-alpha.0, dfir_macro v0.17.0-alpha.0, variadics v1.0.0-alpha.0, variadics_macro v0.8.0-alpha.0, lattices v0.8.0-alpha.0, dfir_pipes v0.1.0-alpha.0, sinktools v0.2.0-alpha.0, hydro_deploy_integration v0.17.0-alpha.0, dfir_rs v0.17.0-alpha.0, hydro_deploy v0.17.0-alpha.0, hydro_lang v0.17.0-alpha.0, hydro_std v0.17.0-alpha.0, safety bump 10 crates ([`12e7666`](https://github.com/hydro-project/hydro/commit/12e76666f7104f81b48de5ddf397b8e72c8a6711))
</details>

## v0.6.0 (2026-05-01)

### New Features (BREAKING)

 - <csr-id-a662ff38541e58bec801644b81b2bfc505779e7b/> use custom `dfir_pipes::Pull` trait [ci-bench]
   This is the pull-half of a big change from using other iterators
   (`std::iter::Iterator` or `futures_core::stream::Stream`) to our own
   `Pull` trait. Key to this more powerful iterator trait is the step enum:
   ```rust
   pub enum Step<Item, Meta, CanPend: Toggle, CanEnd: Toggle> {
   /// An item is ready with associated metadata.
   Ready(Item, Meta),
   /// The pull is not ready yet (only possible when `CanPend = Yes`).
   Pending(CanPend),
   /// The pull has ended (only possible when `CanEnd = Yes`).
   Ended(CanEnd),
   }
   ```
   This abstraction allows `Pull` to represent both synchronous `Iterator`s
   and asynchronous `Stream`s with zero cost. (As well as distinguishing
   between infinite vs finite iterators, which I guess is not actually that
   useful to us). In the future we will also add an `Error` variant
   (#2635). The `Meta` metadata field may be used for full record-level
   tracing (#2242).
   
   This trait has some pseudo-specialization around `Fuse`, and further
   performance improvements may come from true nightly
   `min_specialization`, as well as from converting from `Pusherator/Sink`
   to a new `Push` trait.
   
   Other changes:
   * Moves much of `dfir_rs::compiled::pull` into `dfir_pipes`, using new
   trait
   * Update itertools to `0.14`

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release.
 - 156 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#2618](https://github.com/hydro-project/hydro/issues/2618)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#2618](https://github.com/hydro-project/hydro/issues/2618)**
    - Use custom `dfir_pipes::Pull` trait [ci-bench] ([`a662ff3`](https://github.com/hydro-project/hydro/commit/a662ff38541e58bec801644b81b2bfc505779e7b))
 * **Uncategorized**
    - Release dfir_pipes v0.0.1, example_test v0.0.1, sinktools v0.1.0, hydro_deploy_integration v0.16.0, lattices_macro v0.6.0, variadics_macro v0.7.0, lattices v0.7.0, multiplatform_test v0.7.0, dfir_rs v0.16.0, copy_span v0.1.1, hydro_deploy v0.16.0, hydro_lang v0.16.0, hydro_std v0.16.0 ([`118b356`](https://github.com/hydro-project/hydro/commit/118b356447d92e778313d72a351e5a8d2814aa1a))
    - Release hydro_build_utils v0.1.0, dfir_lang v0.16.0, dfir_macro v0.16.0, variadics v0.1.0, dfir_pipes v0.0.1, example_test v0.0.1, sinktools v0.1.0, hydro_deploy_integration v0.16.0, lattices_macro v0.6.0, variadics_macro v0.7.0, lattices v0.7.0, multiplatform_test v0.7.0, dfir_rs v0.16.0, copy_span v0.1.1, hydro_deploy v0.16.0, hydro_lang v0.16.0, hydro_std v0.16.0, safety bump 13 crates ([`c20757a`](https://github.com/hydro-project/hydro/commit/c20757ae0e9e10463b2a499de4b7d37ab02269d0))
</details>

## v0.5.11 (2025-11-25)

<csr-id-806a6239a649e24fe10c3c90dd30bd18debd41d2/>
<csr-id-dc170e63f62e890bfd0dd054e5a930607fd67545/>

### Bug Fixes

 - <csr-id-c40876ec4bd3b31254d683e479b9a235f3d11f67/> refactor github actions workflows, make stable the default toolchain
 - <csr-id-5ec8b3b9b10b30f3c1b7bd8949874f0b4b7da7e9/> hardcoded crate name issues

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 6 commits contributed to the release.
 - 118 days passed between releases.
 - 4 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 3 unique issues were worked on: [#1977](https://github.com/hydro-project/hydro/issues/1977), [#2028](https://github.com/hydro-project/hydro/issues/2028), [#2283](https://github.com/hydro-project/hydro/issues/2283)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1977](https://github.com/hydro-project/hydro/issues/1977)**
    - Hardcoded crate name issues ([`5ec8b3b`](https://github.com/hydro-project/hydro/commit/5ec8b3b9b10b30f3c1b7bd8949874f0b4b7da7e9))
 * **[#2028](https://github.com/hydro-project/hydro/issues/2028)**
    - Refactor github actions workflows, make stable the default toolchain ([`c40876e`](https://github.com/hydro-project/hydro/commit/c40876ec4bd3b31254d683e479b9a235f3d11f67))
 * **[#2283](https://github.com/hydro-project/hydro/issues/2283)**
    - Exclude crate/module in lib snapshot file names [ci-full] ([`dc170e6`](https://github.com/hydro-project/hydro/commit/dc170e63f62e890bfd0dd054e5a930607fd67545))
 * **Uncategorized**
    - Release sinktools v0.0.1, hydro_deploy_integration v0.15.0, lattices_macro v0.5.11, variadics_macro v0.6.2, lattices v0.6.2, multiplatform_test v0.6.0, dfir_rs v0.15.0, copy_span v0.1.0, hydro_deploy v0.15.0, hydro_lang v0.15.0, hydro_std v0.15.0 ([`ac88df1`](https://github.com/hydro-project/hydro/commit/ac88df1e98af9fa2027488252f6014efa7bef229))
    - Release hydro_build_utils v0.0.1, dfir_lang v0.15.0, dfir_macro v0.15.0, variadics v0.0.10, sinktools v0.0.1, hydro_deploy_integration v0.15.0, lattices_macro v0.5.11, variadics_macro v0.6.2, lattices v0.6.2, multiplatform_test v0.6.0, dfir_rs v0.15.0, copy_span v0.1.0, hydro_deploy v0.15.0, hydro_lang v0.15.0, hydro_std v0.15.0, safety bump 5 crates ([`092de25`](https://github.com/hydro-project/hydro/commit/092de252238dfb9fa6b01e777c6dd8bf9db93398))
    - Ensure `hydro_build_utils` is published in the correct order ([`806a623`](https://github.com/hydro-project/hydro/commit/806a6239a649e24fe10c3c90dd30bd18debd41d2))
</details>

## v0.5.10 (2025-07-30)

<csr-id-3d40d1a65c41dca3893867fb567993a27491fa0c/>

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release.
 - 144 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#1944](https://github.com/hydro-project/hydro/issues/1944)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1944](https://github.com/hydro-project/hydro/issues/1944)**
    - Update `proc-macro-crate` ([`3d40d1a`](https://github.com/hydro-project/hydro/commit/3d40d1a65c41dca3893867fb567993a27491fa0c))
 * **Uncategorized**
    - Release dfir_lang v0.14.0, dfir_macro v0.14.0, hydro_deploy_integration v0.14.0, lattices_macro v0.5.10, variadics_macro v0.6.1, dfir_rs v0.14.0, hydro_deploy v0.14.0, hydro_lang v0.14.0, hydro_optimize v0.13.0, hydro_std v0.14.0, safety bump 6 crates ([`0683595`](https://github.com/hydro-project/hydro/commit/06835950c12884d661100c13f73ad23a98bfad9f))
</details>

## v0.5.9 (2025-03-08)

<csr-id-49a387d4a21f0763df8ec94de73fb953c9cd333a/>
<csr-id-ec3795a678d261a38085405b6e9bfea943dafefb/>

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release.
 - 74 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#1747](https://github.com/hydro-project/hydro/issues/1747)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1747](https://github.com/hydro-project/hydro/issues/1747)**
    - Upgrade to Rust 2024 edition ([`ec3795a`](https://github.com/hydro-project/hydro/commit/ec3795a678d261a38085405b6e9bfea943dafefb))
 * **Uncategorized**
    - Release dfir_lang v0.12.0, dfir_datalog_core v0.12.0, dfir_datalog v0.12.0, dfir_macro v0.12.0, hydroflow_deploy_integration v0.12.0, lattices_macro v0.5.9, variadics v0.0.9, variadics_macro v0.6.0, lattices v0.6.0, multiplatform_test v0.5.0, pusherator v0.0.11, dfir_rs v0.12.0, hydro_deploy v0.12.0, stageleft_macro v0.6.0, stageleft v0.7.0, stageleft_tool v0.6.0, hydro_lang v0.12.0, hydro_std v0.12.0, hydro_cli v0.12.0, safety bump 10 crates ([`973c925`](https://github.com/hydro-project/hydro/commit/973c925e87ed78344494581bd7ce1bbb4186a2f3))
</details>

## v0.5.8 (2024-12-23)

<csr-id-accb13cad718c99d350e4bafe82e0ca38bf94c62/>
<csr-id-2a22d50285ae1be1a5f888d5d15321cc1bb13c82/>

### Documentation

 - <csr-id-204bd117ca3a8845b4986539efb91a0c612dfa05/> add `repository` field to `Cargo.toml`s, fix #1452
   #1452 
   
   Will trigger new releases of the following:
   `unchanged = 'hydroflow_deploy_integration', 'variadics',
   'variadics_macro', 'pusherator'`
   
   (All other crates already have changes, so would be released anyway)

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release.
 - 45 days passed between releases.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 2 unique issues were worked on: [#1501](https://github.com/hydro-project/hydro/issues/1501), [#1623](https://github.com/hydro-project/hydro/issues/1623)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1501](https://github.com/hydro-project/hydro/issues/1501)**
    - Add `repository` field to `Cargo.toml`s, fix #1452 ([`204bd11`](https://github.com/hydro-project/hydro/commit/204bd117ca3a8845b4986539efb91a0c612dfa05))
 * **[#1623](https://github.com/hydro-project/hydro/issues/1623)**
    - Cleanup snapshots ([`2a22d50`](https://github.com/hydro-project/hydro/commit/2a22d50285ae1be1a5f888d5d15321cc1bb13c82))
 * **Uncategorized**
    - Release dfir_lang v0.11.0, dfir_datalog_core v0.11.0, dfir_datalog v0.11.0, dfir_macro v0.11.0, hydroflow_deploy_integration v0.11.0, lattices_macro v0.5.8, variadics v0.0.8, variadics_macro v0.5.6, lattices v0.5.9, multiplatform_test v0.4.0, pusherator v0.0.10, dfir_rs v0.11.0, hydro_deploy v0.11.0, stageleft_macro v0.5.0, stageleft v0.6.0, stageleft_tool v0.5.0, hydro_lang v0.11.0, hydro_std v0.11.0, hydro_cli v0.11.0, safety bump 6 crates ([`361b443`](https://github.com/hydro-project/hydro/commit/361b4439ef9c781860f18d511668ab463a8c5203))
</details>

## v0.5.7 (2024-11-08)

<csr-id-d5677604e93c07a5392f4229af94a0b736eca382/>
<csr-id-014ebb2628b5b80ea1b6426b58c4d62706edb9ef/>

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release.
 - 69 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#1444](https://github.com/hydro-project/hydro/issues/1444)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1444](https://github.com/hydro-project/hydro/issues/1444)**
    - Update pinned rust version, clippy lints, remove some dead code ([`014ebb2`](https://github.com/hydro-project/hydro/commit/014ebb2628b5b80ea1b6426b58c4d62706edb9ef))
 * **Uncategorized**
    - Release hydroflow_lang v0.10.0, hydroflow_datalog_core v0.10.0, hydroflow_datalog v0.10.0, hydroflow_deploy_integration v0.10.0, hydroflow_macro v0.10.0, lattices_macro v0.5.7, variadics v0.0.7, variadics_macro v0.5.5, lattices v0.5.8, multiplatform_test v0.3.0, pusherator v0.0.9, hydroflow v0.10.0, hydro_deploy v0.10.0, stageleft_macro v0.4.0, stageleft v0.5.0, stageleft_tool v0.4.0, hydroflow_plus v0.10.0, hydro_cli v0.10.0, safety bump 8 crates ([`258f480`](https://github.com/hydro-project/hydro/commit/258f4805dbcca36750cbfaaf36db00d3a007d817))
</details>

## v0.5.6 (2024-08-30)

<csr-id-11af32828bab6e4a4264d2635ff71a12bb0bb778/>
<csr-id-2c04f51f1ec44f7898307b6610371dcb490ea686/>

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release.
 - 38 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#1423](https://github.com/hydro-project/hydro/issues/1423)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1423](https://github.com/hydro-project/hydro/issues/1423)**
    - Lower min dependency versions where possible, update `Cargo.lock` ([`2c04f51`](https://github.com/hydro-project/hydro/commit/2c04f51f1ec44f7898307b6610371dcb490ea686))
 * **Uncategorized**
    - Release hydroflow_lang v0.9.0, hydroflow_datalog_core v0.9.0, hydroflow_datalog v0.9.0, hydroflow_deploy_integration v0.9.0, hydroflow_macro v0.9.0, lattices_macro v0.5.6, lattices v0.5.7, multiplatform_test v0.2.0, variadics v0.0.6, pusherator v0.0.8, hydroflow v0.9.0, stageleft_macro v0.3.0, stageleft v0.4.0, stageleft_tool v0.3.0, hydroflow_plus v0.9.0, hydro_deploy v0.9.0, hydro_cli v0.9.0, hydroflow_plus_deploy v0.9.0, safety bump 8 crates ([`1d54331`](https://github.com/hydro-project/hydro/commit/1d54331976040c049e4c97a9fba0e66930efee52))
</details>

## v0.5.5 (2024-07-23)

### Documentation

 - <csr-id-6655f4b44e98f679c049fce8df973531106b428a/> add `#[derive(Lattice)]` docs to README, import into book, fix #1259

### New Features

 - <csr-id-33b9795f207804e9561f228fa0307c5973745241/> add `#[derive(Lattice)]` derive macros, fix #1247
   This adds derive macros to allow user-created macros. Each field must be
   a lattice.
   
   Example usage:
   ```rust
   struct MyLattice<KeySet, Epoch>
   where
   KeySet: Collection,
   Epoch: Ord,
   {
   keys: SetUnion<KeySet>,
   epoch: Max<Epoch>,
   }
   ```
   
   Uses `#[derive(Lattice)]` for the `lattices` library `Pair` lattice.
   Also contains some cleanup in the `lattices` crate.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 2 unique issues were worked on: [#1250](https://github.com/hydro-project/hydro/issues/1250), [#1267](https://github.com/hydro-project/hydro/issues/1267)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1250](https://github.com/hydro-project/hydro/issues/1250)**
    - Add `#[derive(Lattice)]` derive macros, fix #1247 ([`33b9795`](https://github.com/hydro-project/hydro/commit/33b9795f207804e9561f228fa0307c5973745241))
 * **[#1267](https://github.com/hydro-project/hydro/issues/1267)**
    - Add `#[derive(Lattice)]` docs to README, import into book, fix #1259 ([`6655f4b`](https://github.com/hydro-project/hydro/commit/6655f4b44e98f679c049fce8df973531106b428a))
 * **Uncategorized**
    - Release hydroflow_lang v0.8.0, hydroflow_datalog_core v0.8.0, hydroflow_datalog v0.8.0, hydroflow_macro v0.8.0, lattices_macro v0.5.5, lattices v0.5.6, variadics v0.0.5, pusherator v0.0.7, hydroflow v0.8.0, hydroflow_plus v0.8.0, hydro_deploy v0.8.0, hydro_cli v0.8.0, hydroflow_plus_cli_integration v0.8.0, safety bump 7 crates ([`7b9c367`](https://github.com/hydro-project/hydro/commit/7b9c3678930af8010f8e2ffd4069583ece528119))
</details>

