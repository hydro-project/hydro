# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.0.10 (2025-11-25)

### Chore

 - <csr-id-d22507cf6064e4b9f6a02404d58f644cde3dcf0b/> update to rust 1.91.1
 - <csr-id-97426b8a7e3b3af8a58b4c44c768c3f48cd0ed71/> update pinned nightly to 2025-08-20, fix lints

### New Features

 - <csr-id-7efe1dc660ab6c0c68762f71d4359f347cfe73b6/> `sinktools` crate
   this also converts `variadic` to use `#[no_std]`, and adds
   `feature="std"`, and fixes an issue causing trybuild tests to not run
   
   will replace `pusherator` in upcoming PR

### Bug Fixes

 - <csr-id-c40876ec4bd3b31254d683e479b9a235f3d11f67/> refactor github actions workflows, make stable the default toolchain

### Other

 - <csr-id-806a6239a649e24fe10c3c90dd30bd18debd41d2/> ensure `hydro_build_utils` is published in the correct order

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 5 commits contributed to the release.
 - 5 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 4 unique issues were worked on: [#2024](https://github.com/hydro-project/hydro/issues/2024), [#2028](https://github.com/hydro-project/hydro/issues/2028), [#2157](https://github.com/hydro-project/hydro/issues/2157), [#2295](https://github.com/hydro-project/hydro/issues/2295)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#2024](https://github.com/hydro-project/hydro/issues/2024)**
    - Update pinned nightly to 2025-08-20, fix lints ([`97426b8`](https://github.com/hydro-project/hydro/commit/97426b8a7e3b3af8a58b4c44c768c3f48cd0ed71))
 * **[#2028](https://github.com/hydro-project/hydro/issues/2028)**
    - Refactor github actions workflows, make stable the default toolchain ([`c40876e`](https://github.com/hydro-project/hydro/commit/c40876ec4bd3b31254d683e479b9a235f3d11f67))
 * **[#2157](https://github.com/hydro-project/hydro/issues/2157)**
    - `sinktools` crate ([`7efe1dc`](https://github.com/hydro-project/hydro/commit/7efe1dc660ab6c0c68762f71d4359f347cfe73b6))
 * **[#2295](https://github.com/hydro-project/hydro/issues/2295)**
    - Update to rust 1.91.1 ([`d22507c`](https://github.com/hydro-project/hydro/commit/d22507cf6064e4b9f6a02404d58f644cde3dcf0b))
 * **Uncategorized**
    - Ensure `hydro_build_utils` is published in the correct order ([`806a623`](https://github.com/hydro-project/hydro/commit/806a6239a649e24fe10c3c90dd30bd18debd41d2))
</details>

## 0.0.9 (2025-03-08)

<csr-id-49a387d4a21f0763df8ec94de73fb953c9cd333a/>
<csr-id-2fd6aa7417dfa29f389c04c5b9674b80bfed6cf2/>
<csr-id-c293cca6855695107e9cef5c5df99fb04a571934/>

### Chore

 - <csr-id-49a387d4a21f0763df8ec94de73fb953c9cd333a/> upgrade to Rust 2024 edition
   - Updates `Cargo.toml` to use new shared workspace keys
   - Updates lint settings (in workspace `Cargo.toml`)
   - `rustfmt` has changed slightly, resulting in a big diff - there are no
   actual code changes
   - Adds a script to `rustfmt` the template src files
 - <csr-id-2fd6aa7417dfa29f389c04c5b9674b80bfed6cf2/> update pinned nightly to 2025-02-10, cleanups for clippy

### Refactor

 - <csr-id-5cd0a9625822620dcc99b99356edfecbf0549497/> enable lints, cleanups for Rust 2024 #1732

### Chore

 - <csr-id-ec3795a678d261a38085405b6e9bfea943dafefb/> upgrade to Rust 2024 edition
   - Updates `Cargo.toml` to use new shared workspace keys
   - Updates lint settings (in workspace `Cargo.toml`)
   - `rustfmt` has changed slightly, resulting in a big diff - there are no
   actual code changes
   - Adds a script to `rustfmt` the template src files
 - <csr-id-8f4426089dcbbe5d1098f89e367c7be49a03e401/> update pinned nightly to 2025-02-10, cleanups for clippy

### Refactor

 - <csr-id-c293cca6855695107e9cef5c5df99fb04a571934/> enable lints, cleanups for Rust 2024 #1732

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 4 commits contributed to the release.
 - 74 days passed between releases.
 - 3 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 3 unique issues were worked on: [#1704](https://github.com/hydro-project/hydro/issues/1704), [#1737](https://github.com/hydro-project/hydro/issues/1737), [#1747](https://github.com/hydro-project/hydro/issues/1747)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1704](https://github.com/hydro-project/hydro/issues/1704)**
    - Update pinned nightly to 2025-02-10, cleanups for clippy ([`8f44260`](https://github.com/hydro-project/hydro/commit/8f4426089dcbbe5d1098f89e367c7be49a03e401))
 * **[#1737](https://github.com/hydro-project/hydro/issues/1737)**
    - Enable lints, cleanups for Rust 2024 #1732 ([`5cd0a96`](https://github.com/hydro-project/hydro/commit/5cd0a9625822620dcc99b99356edfecbf0549497))
 * **[#1747](https://github.com/hydro-project/hydro/issues/1747)**
    - Upgrade to Rust 2024 edition ([`ec3795a`](https://github.com/hydro-project/hydro/commit/ec3795a678d261a38085405b6e9bfea943dafefb))
 * **Uncategorized**
    - Release dfir_lang v0.12.0, dfir_datalog_core v0.12.0, dfir_datalog v0.12.0, dfir_macro v0.12.0, hydroflow_deploy_integration v0.12.0, lattices_macro v0.5.9, variadics v0.0.9, variadics_macro v0.6.0, lattices v0.6.0, multiplatform_test v0.5.0, pusherator v0.0.11, dfir_rs v0.12.0, hydro_deploy v0.12.0, stageleft_macro v0.6.0, stageleft v0.7.0, stageleft_tool v0.6.0, hydro_lang v0.12.0, hydro_std v0.12.0, hydro_cli v0.12.0, safety bump 10 crates ([`973c925`](https://github.com/hydro-project/hydro/commit/973c925e87ed78344494581bd7ce1bbb4186a2f3))
</details>

## 0.0.8 (2024-12-23)

### Documentation

 - <csr-id-28cd220c68e3660d9ebade113949a2346720cd04/> add `repository` field to `Cargo.toml`s, fix #1452
   #1452 
   
   Will trigger new releases of the following:
   `unchanged = 'hydroflow_deploy_integration', 'variadics',
   'variadics_macro', 'pusherator'`
   
   (All other crates already have changes, so would be released anyway)
 - <csr-id-204bd117ca3a8845b4986539efb91a0c612dfa05/> add `repository` field to `Cargo.toml`s, fix #1452
   #1452 
   
   Will trigger new releases of the following:
   `unchanged = 'hydroflow_deploy_integration', 'variadics',
   'variadics_macro', 'pusherator'`
   
   (All other crates already have changes, so would be released anyway)

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release.
 - 45 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#1501](https://github.com/hydro-project/hydro/issues/1501)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1501](https://github.com/hydro-project/hydro/issues/1501)**
    - Add `repository` field to `Cargo.toml`s, fix #1452 ([`204bd11`](https://github.com/hydro-project/hydro/commit/204bd117ca3a8845b4986539efb91a0c612dfa05))
 * **Uncategorized**
    - Release dfir_lang v0.11.0, dfir_datalog_core v0.11.0, dfir_datalog v0.11.0, dfir_macro v0.11.0, hydroflow_deploy_integration v0.11.0, lattices_macro v0.5.8, variadics v0.0.8, variadics_macro v0.5.6, lattices v0.5.9, multiplatform_test v0.4.0, pusherator v0.0.10, dfir_rs v0.11.0, hydro_deploy v0.11.0, stageleft_macro v0.5.0, stageleft v0.6.0, stageleft_tool v0.5.0, hydro_lang v0.11.0, hydro_std v0.11.0, hydro_cli v0.11.0, safety bump 6 crates ([`361b443`](https://github.com/hydro-project/hydro/commit/361b4439ef9c781860f18d511668ab463a8c5203))
</details>

## 0.0.7 (2024-11-08)

<csr-id-d5677604e93c07a5392f4229af94a0b736eca382/>
<csr-id-47cb703e771f7d1c451ceb9d185ada96410949da/>
<csr-id-656ee328c8710bce7370c851437a80ca3db46a5a/>

### Chore

 - <csr-id-d5677604e93c07a5392f4229af94a0b736eca382/> update pinned rust version, clippy lints, remove some dead code

### Test

 - <csr-id-595ea2555803b9ed7f0a113f399fcdbf0574a317/> ignore trybuild tests inconsistent on latest nightly

### Style

 - <csr-id-cebd1dc35282514f025e047a9b94800f546dd62f/> fixes for nightly clippy
   a couple few spurious `too_many_arguments` and a spurious
   `zombie_processes` still on current nightly (`clippy 0.1.84 (4392847410
   2024-10-21)`)

### Chore

 - <csr-id-014ebb2628b5b80ea1b6426b58c4d62706edb9ef/> update pinned rust version, clippy lints, remove some dead code

### New Features

 - <csr-id-f7e740fb2ba36d0fcf3fd196d60333552911e3a4/> generalized hash trie indexes for relational tuples
   Generalized Hash Tries are part of the SIGMOD '23 FreeJoin
   [paper](https://dl.acm.org/doi/abs/10.1145/3589295) by
   Wang/Willsey/Suciu. They provide a compressed ("factorized")
   representation of relations. By operating in the factorized domain, join
   algorithms can defer cross-products and achieve asymptotically optimal
   performance.
   
   ---------
 - <csr-id-1c2825942f8a326699a7fb68b5372b49918851b5/> additions to variadics including collection types
   adds a number of features:
   
   collection types for variadics (sets, multisets) that allow search via
   RefVars (variadic of refs)
   into_option (convert a variadic to a variadic of options)
   into_vec (convert a variadic to a variadic of vecs)
 - <csr-id-8afd3266dac43c04c3fc29065a13c9c9a6a55afe/> additions to variadics including collection types
   adds a number of features:
   - collection types for variadics (sets, multisets) that allow search via
   RefVars (variadic of refs)
 - <csr-id-48e4eb28a9ce652037ac81b580d30f93159dae9b/> generalized hash trie indexes for relational tuples
   Generalized Hash Tries are part of the SIGMOD '23 FreeJoin
   [paper](https://dl.acm.org/doi/abs/10.1145/3589295) by
   Wang/Willsey/Suciu. They provide a compressed ("factorized")
   representation of relations. By operating in the factorized domain, join
   algorithms can defer cross-products and achieve asymptotically optimal
   performance.
   
   ---------
 - <csr-id-d7a29e252b9c9cae0a4fdf36408da3be78c85caa/> additions to variadics including collection types
   adds a number of features:
   
   collection types for variadics (sets, multisets) that allow search via
   RefVars (variadic of refs)
   into_option (convert a variadic to a variadic of options)
   into_vec (convert a variadic to a variadic of vecs)
 - <csr-id-cffb1d9a969b42736dcd7b72cff2f952c931848a/> additions to variadics including collection types
   adds a number of features:
   - collection types for variadics (sets, multisets) that allow search via
   RefVars (variadic of refs)
   - into_option (convert a variadic to a variadic of options)
   - into_vec (convert a variadic to a variadic of vecs)

### Style

 - <csr-id-47cb703e771f7d1c451ceb9d185ada96410949da/> fixes for nightly clippy
   a couple few spurious `too_many_arguments` and a spurious
   `zombie_processes` still on current nightly (`clippy 0.1.84 (4392847410
   2024-10-21)`)

### Test

 - <csr-id-656ee328c8710bce7370c851437a80ca3db46a5a/> ignore trybuild tests inconsistent on latest nightly

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 8 commits contributed to the release.
 - 69 days passed between releases.
 - 6 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 6 unique issues were worked on: [#1444](https://github.com/hydro-project/hydro/issues/1444), [#1473](https://github.com/hydro-project/hydro/issues/1473), [#1474](https://github.com/hydro-project/hydro/issues/1474), [#1475](https://github.com/hydro-project/hydro/issues/1475), [#1503](https://github.com/hydro-project/hydro/issues/1503), [#1505](https://github.com/hydro-project/hydro/issues/1505)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1444](https://github.com/hydro-project/hydro/issues/1444)**
    - Update pinned rust version, clippy lints, remove some dead code ([`014ebb2`](https://github.com/hydro-project/hydro/commit/014ebb2628b5b80ea1b6426b58c4d62706edb9ef))
 * **[#1473](https://github.com/hydro-project/hydro/issues/1473)**
    - Additions to variadics including collection types ([`cffb1d9`](https://github.com/hydro-project/hydro/commit/cffb1d9a969b42736dcd7b72cff2f952c931848a))
 * **[#1474](https://github.com/hydro-project/hydro/issues/1474)**
    - Revert "feat: additions to variadics including collection types" ([`e493526`](https://github.com/hydro-project/hydro/commit/e4935264015427bf19032632a7f184d1bf6637fd))
 * **[#1475](https://github.com/hydro-project/hydro/issues/1475)**
    - Additions to variadics including collection types ([`d7a29e2`](https://github.com/hydro-project/hydro/commit/d7a29e252b9c9cae0a4fdf36408da3be78c85caa))
 * **[#1503](https://github.com/hydro-project/hydro/issues/1503)**
    - Generalized hash trie indexes for relational tuples ([`48e4eb2`](https://github.com/hydro-project/hydro/commit/48e4eb28a9ce652037ac81b580d30f93159dae9b))
 * **[#1505](https://github.com/hydro-project/hydro/issues/1505)**
    - Fixes for nightly clippy ([`cebd1dc`](https://github.com/hydro-project/hydro/commit/cebd1dc35282514f025e047a9b94800f546dd62f))
 * **Uncategorized**
    - Release hydroflow_lang v0.10.0, hydroflow_datalog_core v0.10.0, hydroflow_datalog v0.10.0, hydroflow_deploy_integration v0.10.0, hydroflow_macro v0.10.0, lattices_macro v0.5.7, variadics v0.0.7, variadics_macro v0.5.5, lattices v0.5.8, multiplatform_test v0.3.0, pusherator v0.0.9, hydroflow v0.10.0, hydro_deploy v0.10.0, stageleft_macro v0.4.0, stageleft v0.5.0, stageleft_tool v0.4.0, hydroflow_plus v0.10.0, hydro_cli v0.10.0, safety bump 8 crates ([`258f480`](https://github.com/hydro-project/hydro/commit/258f4805dbcca36750cbfaaf36db00d3a007d817))
    - Ignore trybuild tests inconsistent on latest nightly ([`595ea25`](https://github.com/hydro-project/hydro/commit/595ea2555803b9ed7f0a113f399fcdbf0574a317))
</details>

## 0.0.6 (2024-08-30)

<csr-id-11af32828bab6e4a4264d2635ff71a12bb0bb778/>

### Chore

 - <csr-id-11af32828bab6e4a4264d2635ff71a12bb0bb778/> lower min dependency versions where possible, update `Cargo.lock`
   Moved from #1418
   
   ---------

### Chore

 - <csr-id-2c04f51f1ec44f7898307b6610371dcb490ea686/> lower min dependency versions where possible, update `Cargo.lock`
   Moved from #1418
   
   ---------

### Bug Fixes

 - <csr-id-43ff49d72789d78535717d2db04cf595cc511274/> allow `PartialEqVariadic::eq_ref` to take `AsRefVar`s with different lifetimes
   Bug found while working on GHTs
 - <csr-id-9646d3e0ffe7d8d3b0bac2c47df9cfe88e3afd1d/> allow `PartialEqVariadic::eq_ref` to take `AsRefVar`s with different lifetimes
   Bug found while working on GHTs

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release.
 - 38 days passed between releases.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 2 unique issues were worked on: [#1367](https://github.com/hydro-project/hydro/issues/1367), [#1423](https://github.com/hydro-project/hydro/issues/1423)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1367](https://github.com/hydro-project/hydro/issues/1367)**
    - Allow `PartialEqVariadic::eq_ref` to take `AsRefVar`s with different lifetimes ([`9646d3e`](https://github.com/hydro-project/hydro/commit/9646d3e0ffe7d8d3b0bac2c47df9cfe88e3afd1d))
 * **[#1423](https://github.com/hydro-project/hydro/issues/1423)**
    - Lower min dependency versions where possible, update `Cargo.lock` ([`2c04f51`](https://github.com/hydro-project/hydro/commit/2c04f51f1ec44f7898307b6610371dcb490ea686))
 * **Uncategorized**
    - Release hydroflow_lang v0.9.0, hydroflow_datalog_core v0.9.0, hydroflow_datalog v0.9.0, hydroflow_deploy_integration v0.9.0, hydroflow_macro v0.9.0, lattices_macro v0.5.6, lattices v0.5.7, multiplatform_test v0.2.0, variadics v0.0.6, pusherator v0.0.8, hydroflow v0.9.0, stageleft_macro v0.3.0, stageleft v0.4.0, stageleft_tool v0.3.0, hydroflow_plus v0.9.0, hydro_deploy v0.9.0, hydro_cli v0.9.0, hydroflow_plus_deploy v0.9.0, safety bump 8 crates ([`1d54331`](https://github.com/hydro-project/hydro/commit/1d54331976040c049e4c97a9fba0e66930efee52))
</details>

## 0.0.5 (2024-07-23)

### New Features

 - <csr-id-20080cb7ceb5b5d3ba349dfd822a37288e40add6/> add traits for dealing with variadics of references
   Renames some traits, but not a breaking change since there hasn't been a
   release that includes those traits.
 - <csr-id-b92dfc7460c985db6935e79d612f42b9b87e746f/> add `iter_any_ref` and `iter_any_mut` to `VariadicsExt`
   Depends on #1241
   
   This isn't needed for the current GHT implementation, but is useful in
   general
 - <csr-id-1a6228f2db081af68890e2e64b3a91f15dd9214f/> add traits for referencing variadics
   This adds a way to convert a reference to a variadic into a variadic of
   references. I.e. `&var_expr!(a, b, c) -> var_expr!(&a, &b, &c)`
 - <csr-id-91259f1eaa0e742a6a10f03306b3aa09c0bcd557/> add traits for dealing with variadics of references
   Renames some traits, but not a breaking change since there hasn't been a
   release that includes those traits.
 - <csr-id-344995854b215ab3257d8355af967f4881c2f437/> add `iter_any_ref` and `iter_any_mut` to `VariadicsExt`
   Depends on #1241
   
   This isn't needed for the current GHT implementation, but is useful in
   general
 - <csr-id-c6b2841f6261932caf4744b86e5e799c9a6e7689/> add traits for referencing variadics
   This adds a way to convert a reference to a variadic into a variadic of
   references. I.e. `&var_expr!(a, b, c) -> var_expr!(&a, &b, &c)`

### Bug Fixes

 - <csr-id-bbef0705d509831415d3bb5ce003116af06b6ffb/> `EitherRefVariadic` is `Variadic`
 - <csr-id-c70114d836e5bc36e2104188867e548e90ab38f4/> fix `HomogenousVariadic` `get` and `get_mut` only returning `None`
 - <csr-id-617e98796dc0359978ba8f487503dbf1317012aa/> `EitherRefVariadic` is `Variadic`
 - <csr-id-fd1104bbdd1c284191088ae77160818db2e91cfd/> fix `HomogenousVariadic` `get` and `get_mut` only returning `None`

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 6 commits contributed to the release.
 - 143 days passed between releases.
 - 5 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 5 unique issues were worked on: [#1241](https://github.com/hydro-project/hydro/issues/1241), [#1245](https://github.com/hydro-project/hydro/issues/1245), [#1324](https://github.com/hydro-project/hydro/issues/1324), [#1325](https://github.com/hydro-project/hydro/issues/1325), [#1352](https://github.com/hydro-project/hydro/issues/1352)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1241](https://github.com/hydro-project/hydro/issues/1241)**
    - Add traits for referencing variadics ([`c6b2841`](https://github.com/hydro-project/hydro/commit/c6b2841f6261932caf4744b86e5e799c9a6e7689))
 * **[#1245](https://github.com/hydro-project/hydro/issues/1245)**
    - Add `iter_any_ref` and `iter_any_mut` to `VariadicsExt` ([`3449958`](https://github.com/hydro-project/hydro/commit/344995854b215ab3257d8355af967f4881c2f437))
 * **[#1324](https://github.com/hydro-project/hydro/issues/1324)**
    - Add traits for dealing with variadics of references ([`91259f1`](https://github.com/hydro-project/hydro/commit/91259f1eaa0e742a6a10f03306b3aa09c0bcd557))
 * **[#1325](https://github.com/hydro-project/hydro/issues/1325)**
    - Fix `HomogenousVariadic` `get` and `get_mut` only returning `None` ([`fd1104b`](https://github.com/hydro-project/hydro/commit/fd1104bbdd1c284191088ae77160818db2e91cfd))
 * **[#1352](https://github.com/hydro-project/hydro/issues/1352)**
    - `EitherRefVariadic` is `Variadic` ([`617e987`](https://github.com/hydro-project/hydro/commit/617e98796dc0359978ba8f487503dbf1317012aa))
 * **Uncategorized**
    - Release hydroflow_lang v0.8.0, hydroflow_datalog_core v0.8.0, hydroflow_datalog v0.8.0, hydroflow_macro v0.8.0, lattices_macro v0.5.5, lattices v0.5.6, variadics v0.0.5, pusherator v0.0.7, hydroflow v0.8.0, hydroflow_plus v0.8.0, hydro_deploy v0.8.0, hydro_cli v0.8.0, hydroflow_plus_cli_integration v0.8.0, safety bump 7 crates ([`7b9c367`](https://github.com/hydro-project/hydro/commit/7b9c3678930af8010f8e2ffd4069583ece528119))
</details>

## 0.0.4 (2024-03-02)

<csr-id-5a451ac4ae75024153a06416fc81d834d1fdae6f/>
<csr-id-7103e77d0da1d73f1c93fcdb260b6a4c9a18ff66/>
<csr-id-b4683450a273d510a11338f07920a5558033b31f/>

### Chore

 - <csr-id-5a451ac4ae75024153a06416fc81d834d1fdae6f/> prep for 0.0.4 release
 - <csr-id-7103e77d0da1d73f1c93fcdb260b6a4c9a18ff66/> update pinned rust to 2024-04-24

### Style

 - <csr-id-894962b540fd67a6ac7fa510e548a903478c62a0/> fix dead code lint

### Chore

 - <csr-id-ae69ce53657104745764fd278153e965182223c4/> prep for 0.0.4 release
 - <csr-id-591fcc99a9b4d7c7cb14a9a0e97d5729834e19c4/> update pinned rust to 2024-04-24

### Style

 - <csr-id-b4683450a273d510a11338f07920a5558033b31f/> fix dead code lint

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 4 commits contributed to the release.
 - 3 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release hydroflow_lang v0.6.0, hydroflow_datalog_core v0.6.0, hydroflow_datalog v0.6.0, hydroflow_macro v0.6.0, lattices v0.5.3, variadics v0.0.4, pusherator v0.0.5, hydroflow v0.6.0, stageleft v0.2.0, hydroflow_plus v0.6.0, hydro_deploy v0.6.0, hydro_cli v0.6.0, hydroflow_plus_cli_integration v0.6.0, safety bump 7 crates ([`0e94db4`](https://github.com/hydro-project/hydro/commit/0e94db41c842c1181574c5e69179027cfa7a19cf))
    - Prep for 0.0.4 release ([`ae69ce5`](https://github.com/hydro-project/hydro/commit/ae69ce53657104745764fd278153e965182223c4))
    - Fix dead code lint ([`894962b`](https://github.com/hydro-project/hydro/commit/894962b540fd67a6ac7fa510e548a903478c62a0))
    - Update pinned rust to 2024-04-24 ([`591fcc9`](https://github.com/hydro-project/hydro/commit/591fcc99a9b4d7c7cb14a9a0e97d5729834e19c4))
</details>

## 0.0.3 (2024-01-29)

<csr-id-1b555e57c8c812bed4d6495d2960cbf77fb0b3ef/>
<csr-id-7e65a08711775656e435e854777c5f089dd31a05/>

### Chore

 - <csr-id-1b555e57c8c812bed4d6495d2960cbf77fb0b3ef/> manually set lockstep-versioned crates (and `lattices`) to version `0.5.1`
   Setting manually since
   https://github.com/frewsxcv/rust-crates-index/issues/159 is messing with
   smart-release

### Refactor

 - <csr-id-3ea3fd5b123b1b088cb94e24fb726ef05114a069/> Improvements prepping for release
   - Adds the "spread"/"splat" `...` syntax to the three variadics macros.
   - Adds `#[sealed]` traits.
   - Adds testing of error messages.
   - Improves docs: `README.md` and Rust docs.

### Chore

 - <csr-id-7c48faf0d8301b498fa59e5eee5cddf5fa341229/> manually set lockstep-versioned crates (and `lattices`) to version `0.5.1`
   Setting manually since
   https://github.com/frewsxcv/rust-crates-index/issues/159 is messing with
   smart-release

### Refactor

 - <csr-id-7e65a08711775656e435e854777c5f089dd31a05/> Improvements prepping for release
   - Adds the "spread"/"splat" `...` syntax to the three variadics macros.
   - Adds `#[sealed]` traits.
   - Adds testing of error messages.
   - Improves docs: `README.md` and Rust docs.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release over the course of 37 calendar days.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#974](https://github.com/hydro-project/hydro/issues/974)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#974](https://github.com/hydro-project/hydro/issues/974)**
    - Improvements prepping for release ([`3ea3fd5`](https://github.com/hydro-project/hydro/commit/3ea3fd5b123b1b088cb94e24fb726ef05114a069))
 * **Uncategorized**
    - Release hydroflow_cli_integration v0.5.1, hydroflow_lang v0.5.1, hydroflow_datalog_core v0.5.1, hydroflow_datalog v0.5.1, hydroflow_macro v0.5.1, lattices v0.5.1, variadics v0.0.3, pusherator v0.0.4, hydroflow v0.5.1, stageleft_macro v0.1.0, stageleft v0.1.0, hydroflow_plus v0.5.1, hydro_deploy v0.5.1, hydro_cli v0.5.1 ([`5a5e6d5`](https://github.com/hydro-project/hydro/commit/5a5e6d5933cf3c20ff23768d4592b0dde94e940b))
    - Manually set lockstep-versioned crates (and `lattices`) to version `0.5.1` ([`7c48faf`](https://github.com/hydro-project/hydro/commit/7c48faf0d8301b498fa59e5eee5cddf5fa341229))
</details>

## 0.0.2 (2023-05-21)

<csr-id-5a3c2949653685de1e33cf7412057a70880283df/>

### Style

 - <csr-id-5a3c2949653685de1e33cf7412057a70880283df/> rustfmt format code comments

### Style

 - <csr-id-92e17e59de26473f99fd83454668045aaddc691a/> rustfmt format code comments

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#660](https://github.com/hydro-project/hydro/issues/660)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#660](https://github.com/hydro-project/hydro/issues/660)**
    - Rustfmt format code comments ([`92e17e5`](https://github.com/hydro-project/hydro/commit/92e17e59de26473f99fd83454668045aaddc691a))
 * **Uncategorized**
    - Release hydroflow_cli_integration v0.0.1, hydroflow_lang v0.0.1, hydroflow_datalog_core v0.0.1, hydroflow_datalog v0.0.1, hydroflow_macro v0.0.1, lattices v0.1.0, variadics v0.0.2, pusherator v0.0.1, hydroflow v0.0.2 ([`d91ebc9`](https://github.com/hydro-project/hydro/commit/d91ebc9e8e23965089c929558a09fc430ee72f2c))
</details>

## 0.0.1 (2023-04-25)

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 4 commits contributed to the release.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#617](https://github.com/hydro-project/hydro/issues/617)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#617](https://github.com/hydro-project/hydro/issues/617)**
    - Update `Cargo.toml`s for publishing ([`3a08bb2`](https://github.com/hydro-project/hydro/commit/3a08bb2501482323e069c6c1f808d611ac679f1f))
 * **Uncategorized**
    - Setup release workflow ([`f4eb56d`](https://github.com/hydro-project/hydro/commit/f4eb56dacebe96a92cb7448bcce14b8b5093c9d5))
    - Rename variadics/tuple_list macros ([`d443697`](https://github.com/hydro-project/hydro/commit/d4436975b85542bd62e862fdcefcd7249f5a732e))
    - Rename pkg `type_list` -> `variadics` ([`30777e2`](https://github.com/hydro-project/hydro/commit/30777e2608c72a3353733ef353373914b79407e2))
</details>

