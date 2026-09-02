//! Meta-test for host/dylib dependency-feature divergence in the simulator.
//!
//! Tracked in <https://github.com/hydro-project/hydro/issues/3183>.
//!
//! # The bug
//!
//! The simulator compiles staged dataflow into a `cdylib` via a synthetic
//! cargo workspace under `target/hydro_trybuild/<crate>/`, `dlopen`s it, and
//! shares Rust data structures across the boundary: the generated dylib code
//! creates the `tokio::sync::mpsc` unbounded channels backing every
//! `sim_input`/`sim_output` port and hands the `UnboundedReceiver<Bytes>`
//! across the `dlopen` boundary for the host's `SimReceiver` to poll. Rust
//! has no stable ABI, so this is only sound if the host test binary and the
//! dylib are compiled with *identical* dependency configurations.
//!
//! `create_trybuild()` (in `hydro_lang/src/compile/trybuild/generate.rs`)
//! synthesizes the dylib workspace's manifest from **the staged crate's
//! `Cargo.toml` alone** (`dependencies::get_manifest(&source_dir)` +
//! `Runner::make_manifest`, which merges just that crate's `[dependencies]`
//! and `[dev-dependencies]`). Defenses exist for *version* skew — the
//! workspace `Cargo.lock` is copied and `cargo update -w` is run — but the
//! trybuild workspace re-runs **feature resolution** over its own small
//! graph, and `features::find()` only forwards the staged crate's *own
//! package features* (from the test binary's fingerprint JSON), not
//! transitive dependency features.
//!
//! Meanwhile the host test binary is built by the user's cargo invocation,
//! where features unify across *every crate in the invocation*. Any feature
//! a **sibling workspace crate** enables on a shared dependency makes the
//! host's copy of that dependency cfg-diverge from the dylib's copy — with
//! no manifest change to the staged crate at all.
//!
//! # This test
//!
//! Hydro's own workspace cannot exhibit the bug: `hydro_test`,
//! `hydro_test_embedded`, and the project template all carry a
//! `tokio = { features = ["full"] }` dev-dependency, which
//! `Runner::make_manifest` copies into the trybuild manifest — accidentally
//! re-aligning the dylib with the feature-unified host. So this test
//! scaffolds a fresh two-crate workspace (under the target directory) where
//! that masking is absent:
//!
//! - `staged`: a minimal staged dataflow crate (source in
//!   `sim_feature_unification/staged_lib.rs`) whose only tokio dependency is
//!   featureless. Its `host_and_dylib_agree_on_tokio_layout` test streams
//!   `size_of::<tokio::runtime::Handle>()` from inside the sim dylib out a
//!   sim port and asserts it matches the host's value — a deterministic,
//!   platform-independent witness of the divergence (no reliance on the UB
//!   manifesting as a crash).
//! - `sibling`: an empty crate whose only purpose is
//!   `tokio = { version = "1", features = ["full"] }`.
//!
//! and drives `cargo test` in it with the two invocation shapes:
//!
//! | scaffold invocation             | expected | today                        |
//! |---------------------------------|----------|------------------------------|
//! | `cargo test -p sim_repro_staged`| pass     | passes (no unification)      |
//! | `cargo test --workspace`        | pass     | **fails** (host tokio gains `full`; dylib stays minimal) |
//!
//! With tokio v1.53.1 the unified host gains `full, parking_lot, process,
//! rt-multi-thread, signal, signal-hook-registry` over the dylib;
//! `rt-multi-thread` alone changes public type layouts
//! (`runtime::Handle` 8 → 16, `Runtime` 64 → 80, `time::Sleep` 96 → 112,
//! `net::TcpListener` 32 → 40). In the originating report
//! (hydro-project/infinity#112, an equivalent workspace shape) the same
//! divergence manifested as a deterministic SIGSEGV on macOS/aarch64: the
//! host polls a dylib-created `UnboundedReceiver<Bytes>` and dies in
//! `tokio::sync::mpsc::block::Block::<Bytes>::is_at_index` with
//! `self == NULL`, via `SimReceiver::next` →
//! `UnboundedReceiverStream::poll_next` → `Rx::pop` → `try_advancing_head`.
//! On Linux/x86_64 the sim tests pass silently under the same divergence —
//! the layout witness test fails either way.
//!
//! # Fix directions
//!
//! - Resolve the trybuild workspace's features from the *host invocation's*
//!   resolved feature set (e.g. via `cargo metadata`/unit-graph of the
//!   running build) rather than from the staged crate's manifest alone.
//! - Or detect the divergence at sim-compile time (compare resolved features
//!   of shared boundary crates — tokio, bytes, dfir_rs — between host and
//!   trybuild) and fail with an actionable error instead of UB.

#![cfg(feature = "sim")]

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

/// Serializes scaffold access between the tests in this binary (nextest runs
/// them in separate processes, but plain `cargo test` uses threads).
static SCAFFOLD_LOCK: Mutex<()> = Mutex::new(());

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
stageleft = "0.15.1"

# Used by `host_and_dylib_agree_on_tokio_layout` to compare
# `size_of::<tokio::runtime::Handle>()` between the host test binary and the
# generated sim dylib. Must be a regular dependency (not a dev-dependency) so
# that stageleft re-exports it through `__staged::__deps` for the `q!()` body
# that runs inside the dylib. Deliberately requests **no features**: this
# manifest is what the generated trybuild workspace resolves from, so pinning
# tokio features here would re-align the dylib with the unified host and mask
# the bug -- exactly what `hydro_test`'s `tokio = { features = ["full"] }`
# dev-dependency does accidentally.
tokio = { version = "1", default-features = false }

[build-dependencies]
stageleft_tool = "0.15.1"

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

# This crate exists solely to unify extra tokio features into any cargo
# invocation that includes it (e.g. `cargo test --workspace`). It contains no
# code. With cargo's feature unification, the HOST test binary of
# `sim_repro_staged` is then built against a tokio compiled with these
# features, while the sim dylib generated under `target/hydro_trybuild/` is
# still built against a minimal-feature tokio (its manifest is synthesized
# from `sim_repro_staged`'s Cargo.toml alone).
[dependencies]
tokio = { version = "1", features = ["full"] }
"#,
    );
    write(
        "sibling/src/lib.rs",
        "//! Intentionally empty; exists only to feature-unify `tokio/full` into\n\
         //! the host build. See the workspace root and the meta-test docs.\n",
    );

    // Seed the scaffold's lockfile from the repo's, mirroring the version-skew
    // defense in `create_trybuild()` (which copies the workspace `Cargo.lock`
    // into the generated trybuild workspace). This pins the scaffold's
    // dependency *versions* to the ones the repo already builds with — the
    // divergence under test is purely about *features*, which the lockfile
    // does not constrain.
    let scaffold_lock = root.join("Cargo.lock");
    if !scaffold_lock.exists() {
        std::fs::copy(
            hydro_lang_dir.parent().unwrap().join("Cargo.lock"),
            scaffold_lock,
        )
        .unwrap();
    }

    root
}

/// Runs `cargo test <args>` in the scaffolded workspace, capturing output.
fn scaffold_cargo_test(root: &Path, args: &[&str]) -> std::process::Output {
    Command::new("cargo")
        .arg("test")
        .args(args)
        .current_dir(root)
        // Keep the scaffold's build products inside the scaffold, both for
        // isolation from the outer build and so repeat runs are cached.
        .env("CARGO_TARGET_DIR", root.join("target"))
        .env("NO_COLOR", "1")
        .output()
        .unwrap()
}

#[track_caller]
fn assert_scaffold_tests_pass(invocation: &[&str], output: &std::process::Output) {
    assert!(
        output.status.success(),
        "`cargo test {}` in the scaffolded repro workspace failed ({}).\n\
         The failure output below should show the host test binary and the \
         sim dylib disagreeing about tokio's configuration (see this file's \
         module docs).\n\n=== stdout ===\n{}\n=== stderr ===\n{}",
        invocation.join(" "),
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}

/// Sanity direction: with no sibling crate in the invocation, the host and
/// the sim dylib resolve identical dependency features, so the scaffold's
/// tests (including the layout witness) pass. This also keeps the scaffold
/// machinery itself from rotting while the companion test below is ignored.
#[test]
fn sim_dylib_matches_host_for_solo_invocation() {
    let _guard = SCAFFOLD_LOCK.lock().unwrap();
    let root = scaffold_workspace();
    let output = scaffold_cargo_test(&root, &["-p", "sim_repro_staged"]);
    assert_scaffold_tests_pass(&["-p", "sim_repro_staged"], &output);
}

/// The bug: a sibling workspace crate feature-unifies `tokio/full` into the
/// host test binary, the generated sim dylib stays minimal, and the two
/// disagree about the layout of types they share across the `dlopen`
/// boundary (UB; SIGSEGV observed on macOS/aarch64). The scaffold's
/// `host_and_dylib_agree_on_tokio_layout` test fails deterministically on
/// every platform.
#[test]
#[ignore = "known bug: sim dylib dependency features diverge from the host under workspace \
            feature unification (https://github.com/hydro-project/hydro/issues/3183); un-ignore \
            when create_trybuild() resolves features from the host invocation"]
fn sim_dylib_matches_host_under_workspace_feature_unification() {
    let _guard = SCAFFOLD_LOCK.lock().unwrap();
    let root = scaffold_workspace();
    let output = scaffold_cargo_test(&root, &["--workspace"]);
    assert_scaffold_tests_pass(&["--workspace"], &output);
}
