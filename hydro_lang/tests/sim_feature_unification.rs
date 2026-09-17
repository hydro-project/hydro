//! Meta-test: the simulator's `dlopen` boundary must be insensitive to
//! host/dylib dependency-feature divergence.
//!
//! Tracked in <https://github.com/hydro-project/hydro/issues/3183>.
//!
//! # The boundary
//!
//! The simulator compiles staged dataflow into a `cdylib` via a synthetic
//! cargo workspace under `target/hydro_trybuild/<crate>/`, `dlopen`s it, and
//! calls into it. Test-side values never cross that boundary as Rust values:
//! every `sim_input` / `sim_output` port carries bincode-serialized `Bytes`.
//! What *does* cross raw is the machinery around the ports: the
//! `dfir_rs::util::unsync::mpsc` channels backing them (created inside the
//! dylib, polled by the host's `SimReceiver`), the erased DFIR graphs, and the
//! hook trait objects. Rust has no stable ABI, so this is only sound if the
//! host test binary and the dylib agree on the layout of *those* types.
//!
//! # Why the two sides can disagree
//!
//! The host and the dylib are separate cargo resolutions. `create_trybuild()`
//! (in `hydro_lang/src/compile/trybuild/generate.rs`) synthesizes the dylib
//! workspace's manifest from **the staged crate's `Cargo.toml` alone**
//! (`dependencies::get_manifest(&source_dir)` + `Runner::make_manifest`).
//! Versions are pinned by copying the workspace `Cargo.lock`, but the trybuild
//! workspace re-runs **feature resolution** over its own small graph, and
//! `features::find()` only forwards the staged crate's *own package features*
//! (from the test binary's fingerprint JSON), not transitive dependency
//! features. The host test binary, meanwhile, is built by the user's cargo
//! invocation, where features unify across *every crate in the invocation*:
//! any feature a sibling workspace crate enables on a shared dependency
//! reaches the host's copy but not the dylib's — with no change to the staged
//! crate at all.
//!
//! Making the two resolutions agree is not practical (the host's unified set
//! depends on which crates the user happened to pass to cargo), so the
//! invariant Hydro maintains instead is that **the boundary is built only
//! from types whose layout is pinned on both sides**:
//!
//! - `std` types, whose layout is fixed by the toolchain both sides share;
//! - Hydro-owned crates whose layout-affecting features are enabled
//!   explicitly by *both* `hydro_lang/sim` (host) and `hydro_lang/sim_runtime`
//!   (dylib) — see the `dfir_rs/meta` / `dfir_rs/tokio` entries in
//!   `hydro_lang/Cargo.toml`.
//!
//! Third-party crates with layout-changing features are not allowed on the
//! boundary. Two have been removed so far: the ports were `tokio::sync::mpsc`
//! channels until #3113 (in 0.17.0-alpha.4 a sibling's `tokio/full` changed
//! the channel internals' layout and produced a deterministic SIGSEGV on
//! macOS/aarch64 — hydro-project/infinity#112), and the unsync channel's
//! `Shared<T>` held a `SmallVec<[Waker; 1]>` whose layout flips under
//! `smallvec/union` (32 -> 24 bytes) until it was replaced with a `std`-only
//! inline-first waker store. With the `SmallVec` still in place, the
//! `--workspace` invocation below aborts the scaffold's test binary on
//! Linux/x86_64 ("thread caused non-unwinding panic", SIGABRT).
//!
//! # This test
//!
//! Hydro's own workspace cannot exhibit divergence: `hydro_test`,
//! `hydro_test_embedded`, and the project template all carry a
//! `tokio = { features = ["full"] }` dev-dependency, which
//! `Runner::make_manifest` copies into the trybuild manifest — accidentally
//! re-aligning the dylib with the feature-unified host. So this test
//! scaffolds a fresh two-crate workspace (under the target directory) where
//! that masking is absent:
//!
//! - `staged`: a minimal staged dataflow crate (source in
//!   `sim_feature_unification/staged_lib.rs`) whose dependencies request no
//!   extra features. Its `add_one_adds_one` test round-trips values through
//!   both sim ports; its `smallvec_layout_diverges_only_under_workspace_unification`
//!   test streams `size_of::<SmallVec<[u64; 2]>>()` from inside the dylib and
//!   compares it with the host's value, which pins the *premise* of each
//!   invocation (aligned vs. diverged dependencies) so that the workspace run
//!   cannot pass vacuously.
//! - `sibling`: an empty crate whose only purpose is
//!   `tokio = { features = ["full"] }` and `smallvec = { features = ["union"] }`
//!   — the two layout-changing features of the crates that used to sit on
//!   the boundary.
//!
//! and drives `cargo test` in it with the two invocation shapes:
//!
//! | scaffold invocation              | host vs. dylib deps | sim tests |
//! |----------------------------------|---------------------|-----------|
//! | `cargo test -p sim_repro_staged` | aligned             | pass      |
//! | `cargo test --workspace`         | diverged            | pass      |
//!
//! A layout mismatch on the boundary is undefined behavior, not a guaranteed
//! crash (the tokio-era mismatch passed silently on Linux/x86_64 and crashed
//! on macOS/aarch64), so a green workspace run here is a smoke test, not a
//! proof. The premise check is what makes a regression *observable* when the
//! UB does manifest: it rules out "the run passed because the feature sets
//! happened to line up".

#![cfg(feature = "sim")]

use std::path::{Path, PathBuf};
use std::process::Command;

/// Read by the scaffold's `smallvec_layout_diverges_only_under_workspace_unification`
/// test (see `sim_feature_unification/staged_lib.rs`) to pick the expected
/// direction of the host/dylib comparison.
const DIVERGENCE_ENV: &str = "SIM_REPRO_EXPECT_HOST_DYLIB_DIVERGENCE";

/// TOML-safe absolute path (backslashes are escapes in basic TOML strings).
fn toml_path(path: &Path) -> String {
    path.display().to_string().replace('\\', "/")
}

/// Writes the repro workspace under the target directory (stable location so
/// repeat runs reuse the scaffold's build cache) and returns its root.
fn scaffold_workspace() -> PathBuf {
    let hydro_lang_dir = Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf();
    let target_dir = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| hydro_lang_dir.parent().unwrap().join("target"));
    let root = target_dir.join("sim_feature_unification_scaffold");

    let hydro_lang_path = toml_path(&hydro_lang_dir);

    let write = |rel: &str, contents: &str| {
        let path = root.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, contents).unwrap();
    };

    write(
        "Cargo.toml",
        r#"# Generated by hydro_lang/tests/sim_feature_unification.rs -- do not edit.
[workspace]
members = ["staged", "sibling"]
resolver = "2"
"#,
    );

    write(
        "staged/Cargo.toml",
        &r#"# Generated by hydro_lang/tests/sim_feature_unification.rs -- do not edit.
[package]
name = "sim_repro_staged"
publish = false
version = "0.0.0"
edition = "2024"

[dependencies]
hydro_lang = { path = "__HYDRO_LANG__" }
# `*` (as in `template/hydro/Cargo.toml`) so the seeded lockfile decides:
# the staged crate must use the same stageleft as the `hydro_lang` it links.
stageleft = "*"

# Layout witness for the premise check (see the meta-test's module docs).
# Must be a regular dependency (not a dev-dependency) so that stageleft
# re-exports it through `__staged::__deps` for the `q!()` body that runs
# inside the dylib. Deliberately requests **no features**: this manifest is
# what the generated trybuild workspace resolves from, so pinning `union`
# here would re-align the dylib with the unified host and defeat the check.
smallvec = { version = "1", default-features = false }

[build-dependencies]
stageleft_tool = "*"

[dev-dependencies]
hydro_lang = { path = "__HYDRO_LANG__", features = ["sim"] }
"#
        .replace("__HYDRO_LANG__", &hydro_lang_path),
    );
    write(
        "staged/build.rs",
        "fn main() {\n    stageleft_tool::gen_final!();\n}\n",
    );
    write(
        "staged/src/lib.rs",
        include_str!("sim_feature_unification/staged_lib.rs"),
    );

    write(
        "sibling/Cargo.toml",
        r#"# Generated by hydro_lang/tests/sim_feature_unification.rs -- do not edit.
[package]
name = "sim_repro_sibling"
publish = false
version = "0.0.0"
edition = "2024"

# This crate exists solely to unify extra features into any cargo invocation
# that includes it (e.g. `cargo test --workspace`). It contains no code. With
# cargo's feature unification, the HOST test binary of `sim_repro_staged` is
# then built against tokio and smallvec compiled with these features, while
# the sim dylib generated under `target/hydro_trybuild/` is still built
# against the minimal-feature copies (its manifest is synthesized from
# `sim_repro_staged`'s Cargo.toml alone). Both features change type layouts,
# and both crates used to provide types on the dlopen boundary.
[dependencies]
tokio = { version = "1", features = ["full"] }
smallvec = { version = "1", features = ["union"] }
"#,
    );
    write(
        "sibling/src/lib.rs",
        "//! Intentionally empty; exists only to feature-unify `tokio/full` and\n\
         //! `smallvec/union` into the host build. See the workspace root and the\n\
         //! meta-test docs.\n",
    );

    // Seed the scaffold's lockfile from the repo's, mirroring the version-skew
    // defense in `create_trybuild()` (which copies the workspace `Cargo.lock`
    // into the generated trybuild workspace). This pins the scaffold's
    // dependency *versions* to the ones the repo already builds with — the
    // divergence under test is purely about *features*, which the lockfile
    // does not constrain. Copied on every run so the scaffold follows the
    // repo when it bumps a dependency.
    std::fs::copy(
        hydro_lang_dir.parent().unwrap().join("Cargo.lock"),
        root.join("Cargo.lock"),
    )
    .unwrap();

    root
}

/// Runs `cargo test <args>` in the scaffolded workspace, capturing output.
fn scaffold_cargo_test(
    root: &Path,
    args: &[&str],
    expect_divergence: bool,
) -> std::process::Output {
    let mut command = Command::new("cargo");
    command
        .arg("test")
        .args(args)
        .current_dir(root)
        // Keep the scaffold's build products inside the scaffold, both for
        // isolation from the outer build and so repeat runs are cached.
        .env("CARGO_TARGET_DIR", root.join("target"))
        .env("NO_COLOR", "1")
        .env_remove(DIVERGENCE_ENV);
    if expect_divergence {
        command.env(DIVERGENCE_ENV, "1");
    }
    command.output().unwrap()
}

#[track_caller]
fn assert_scaffold_tests_pass(invocation: &[&str], output: &std::process::Output) {
    assert!(
        output.status.success(),
        "`cargo test {}` in the scaffolded repro workspace failed ({}).\n\
         Either the sim ports broke under host/dylib dependency-feature \
         divergence, or the premise check found the two sides unexpectedly \
         (un)aligned (see this file's module docs).\n\n=== stdout ===\n{}\n=== stderr ===\n{}",
        invocation.join(" "),
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}

/// Runs both invocation shapes against one scaffold (a single test, so the
/// scaffold's files and build cache are never touched concurrently).
///
/// - `-p sim_repro_staged` is the baseline: the host and the sim dylib
///   resolve identical dependency features, the premise check asserts the
///   alignment, and the port round-trip passes.
/// - `--workspace` is the scenario from #3183: the sibling crate
///   feature-unifies `tokio/full` and `smallvec/union` into the host test
///   binary while the generated sim dylib stays minimal. The premise check
///   asserts that the divergence is real, and the port round-trip must still
///   pass because nothing on the `dlopen` boundary depends on those features.
#[test]
fn sim_ports_work_regardless_of_host_feature_unification() {
    let root = scaffold_workspace();

    let solo = ["-p", "sim_repro_staged"];
    let output = scaffold_cargo_test(&root, &solo, false);
    assert_scaffold_tests_pass(&solo, &output);

    let workspace = ["--workspace"];
    let output = scaffold_cargo_test(&root, &workspace, true);
    assert_scaffold_tests_pass(&workspace, &output);
}
