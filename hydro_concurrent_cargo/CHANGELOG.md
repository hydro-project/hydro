

## v0.1.2-alpha.0 (2026-09-30)

## v0.1.1-alpha.0 (2026-09-21)

### New Features

 - <csr-id-efbf2c994f769fa7320d2773a70f95831b984c02/> make trybuild final compiles actually link dynamically [ci-full]
   Adds `scripts/bench_trybuild.sh` to benchmark the per-test "final
   compile" of
   trybuild-generated examples (the ~6s-per-test cost in CI even with a
   fully warm
   cache), and fixes two bugs that were silently defeating the
   dynamic-linking
   design, cutting the final compile from ~4.2s to ~1.25s locally (3.3x).
   
   Root causes found via the benchmark:
   
   1. Generated examples statically linked the entire dependency graph. The
   `dylib-examples` crate had *both* the base trybuild crate and the dylib
   as
   dev-dependencies. Since generated examples reference the base crate by
   name
   (`use {crate}_hydro_trybuild::...`), rustc linked the base rlib (and its
   ~286 transitive rlibs, ~420MB) statically into every example; the dylib
   was
   linked but then dropped by `--as-needed`. Fix: `dylib-examples` now
   depends
      *only* on the dylib crate, renamed to the base crate's package name
   (`{name} = { package = "{name}-dylib", path = "../dylib" }`), so the
   extern
      name resolves to the dylib and the final link pulls in just
      libhydro_..._dylib.so + shared libstd + compiler_builtins.
   
   2. Prebuild "poisoned" the dylib with statically-linked libstd. Cargo
   passes
      `-C prefer-dynamic` to dylib crates only when they are built as
   *dependencies*, not as the primary build target — and both variants
   share a
   cargo fingerprint. The prebuild built the dylib crate directly, caching
   a
   static-libstd variant that fails to link into examples ("cannot satisfy
      dependencies so `std` only shows up once"). Fix (both in
   hydro_lang::compile::trybuild::generate and hydro_deploy
   rust_crate/build.rs):
   prebuild `dylib-examples --lib` instead, which builds the dylib
   transitively
   as a dependency. A version comment in the generated dylib `lib.rs` busts
   the
      fingerprint of existing caches holding the poisoned variant.
   
   Supporting changes:
   - Since examples are now genuinely dynamic, bake additional rpath
   entries
   (debug/deps for the dylib, `rustc --print target-libdir` for shared
   libstd)
   into sim/maelstrom artifacts, and extend rpath handling to macOS. On
   macOS
   the rpaths use raw ld64 syntax (`-rpath <path>` as two link-args)
   because
     rustc may invoke `rust-lld -flavor darwin` directly, which rejects
     `-Wl,`-wrapped arguments; the clang driver also forwards this form.
   - Include the dylib/dylib-examples manifests in the trybuild
   cache-invalidation
   hash so existing target dirs regenerate their Cargo.lock (dep graph
   changed).
   - Instrument `compile_trybuild_example` with `tracing` spans (target
   `hydro_build`: prebuild / populate_job_dir / final_build, plus a debug
   event
   with the exact final cargo command). hydro_lang's test-init ctor now
   installs
     the telemetry tracing subscriber, so the spans are opt-in via RUST_LOG
   (silent by default). All ad-hoc `[hydro-build]` eprintln logging is
   removed
   (hydro_lang, hydro_deploy, hydro_concurrent_cargo); the
   build-coordination.log
     mechanism is unchanged.
   - `scripts/bench_trybuild.sh`: warms via two hydro_lang sim tests with
   RUST_LOG=hydro_build=debug, captures the exact final `cargo rustc`
   commands,
   and replays them N times (touching generated sources to force
   recompiles,
   mimicking CI). Knobs: ITERS, BENCH_TESTS, LIB_METADATA=0,
   EXTRA_RUSTC_FLAGS,
   EXTRA_CARGO_FLAGS; `--nextest` mode measures end-to-end through the test
     harness using the span-close timings.
   
   Measured (32-core Linux, warm cache): baseline 4.23s mean per final
   compile;
   after fixes 1.25s (~0.35s cargo freshness check, ~0.45s rustc
   frontend+codegen of the generated example, ~0.44s link). lld and
   `-Cdebuginfo=0` were also benchmarked and showed no further gain.
   Results are
   identical with and without `__CARGO_DEFAULT_LIB_METADATA=1`.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release.
 - 69 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#3040](https://github.com/hydro-project/hydro/issues/3040)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#3040](https://github.com/hydro-project/hydro/issues/3040)**
    - Make trybuild final compiles actually link dynamically [ci-full] ([`efbf2c9`](https://github.com/hydro-project/hydro/commit/efbf2c994f769fa7320d2773a70f95831b984c02))
 * **Uncategorized**
    - Release hydro_build_utils v0.1.1-alpha.1, dfir_lang v0.17.0-alpha.4, dfir_macro v0.17.0-alpha.4, variadics v0.2.0-alpha.3, variadics_macro v0.8.0-alpha.2, lattices v0.8.0-alpha.4, example_test v0.0.2-alpha.0, sinktools v0.2.0-alpha.4, hydro_deploy_integration v0.17.0-alpha.3, dfir_rs v0.17.0-alpha.5, copy_span v0.1.2-alpha.0, hydro_concurrent_cargo v0.1.1-alpha.0, hydro_deploy v0.17.0-alpha.4, hydro_lang v0.17.0-alpha.5, hydro_std v0.17.0-alpha.5, safety bump 4 crates ([`38ccb27`](https://github.com/hydro-project/hydro/commit/38ccb27ae7a08b9ac1ab544f4047a4f4592ee230))
</details>

## v0.1.0-alpha.0 (2026-07-14)

### Chore

 - <csr-id-b4da85e789d7c97d183eef3f3d7915b5e7bc24f8/> prepare for release

### New Features

 - <csr-id-c128c2293c7b5c780e1c892d5a44505781b5a211/> parallel compilation with per-job target dirs and shared artifact symlinks [ci-full]
   Each compilation job gets its own `--target-dir` (under
   `{target}/jobs/{name}`) to
   avoid cargo's global artifact-dir lock, while sharing compiled artifacts
   via symlinks
   to `.fingerprint`, `build`, and `deps` in the shared target dir.
   
   A prebuild step compiles the dylib crate (or --lib for non-dylib) into
   the shared
   target dir before the parallel final builds start, ensuring all
   dependencies are ready.
   
   Also adds feature forwarding to the generated dylib crate's Cargo.toml.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release over the course of 17 calendar days.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#2975](https://github.com/hydro-project/hydro/issues/2975)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#2975](https://github.com/hydro-project/hydro/issues/2975)**
    - Parallel compilation with per-job target dirs and shared artifact symlinks [ci-full] ([`c128c22`](https://github.com/hydro-project/hydro/commit/c128c2293c7b5c780e1c892d5a44505781b5a211))
 * **Uncategorized**
    - Release dfir_lang v0.17.0-alpha.3, variadics v0.2.0-alpha.2, lattices v0.8.0-alpha.3, dfir_pipes v0.1.0-alpha.3, multiplatform_test v0.7.1-alpha.0, dfir_rs v0.17.0-alpha.4, hydro_concurrent_cargo v0.1.0-alpha.0, hydro_deploy v0.17.0-alpha.3, hydro_lang v0.17.0-alpha.4, hydro_std v0.17.0-alpha.4, safety bump 3 crates ([`6287d84`](https://github.com/hydro-project/hydro/commit/6287d84c83b0a37798d6afdeb6bfacaf9a8ce3d1))
    - Prepare for release ([`b4da85e`](https://github.com/hydro-project/hydro/commit/b4da85e789d7c97d183eef3f3d7915b5e7bc24f8))
</details>

