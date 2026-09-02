//! Minimal staged crate reproducing host/dylib dependency-feature divergence
//! in Hydro's simulator.
//!
//! The simulator (`flow.sim()`) compiles staged dataflow into a `cdylib` via a
//! synthetic cargo workspace under `target/hydro_trybuild/<crate>/`, `dlopen`s
//! it, and shares Rust data structures across the boundary (e.g. the
//! `tokio::sync::mpsc` unbounded channels backing `sim_input`/`sim_output`
//! ports are created inside the dylib and polled by the host's
//! `SimReceiver`). That is only sound if the host test binary and the dylib
//! are compiled with *identical* dependency configurations.
//!
//! `create_trybuild()` (in `hydro_lang/src/compile/trybuild/generate.rs`)
//! synthesizes the dylib workspace's manifest from **this crate's
//! `Cargo.toml` alone** and re-runs feature resolution over that small graph.
//! Dependency *versions* are pinned (the workspace `Cargo.lock` is copied and
//! `cargo update -w` is run), but dependency *features* are not: features
//! that a **sibling workspace crate** unifies into the host build (here,
//! `../sibling`'s `tokio = { features = ["full"] }`) never reach the dylib's
//! manifest. `features::find()` only forwards this crate's own package
//! features, not transitive dependency features.
//!
//! Reproduction (see README.md):
//!
//! ```text
//! cargo test -p sim_repro_staged   # host tokio == dylib tokio -> passes
//! cargo test --workspace           # host tokio gains "full" -> UB
//! ```
//!
//! The resulting UB has been observed as a deterministic SIGSEGV on
//! macOS/aarch64 (`tokio::sync::mpsc::block::Block::<Bytes>::is_at_index`
//! called with `self == NULL` while the host polls a dylib-created
//! `UnboundedReceiver<Bytes>`); on Linux/x86_64 the same divergence has been
//! observed to pass silently. Both are the same soundness bug — the crash is
//! just where the dice land. `check_feature_divergence.sh` demonstrates the
//! divergence deterministically on any platform.

#[cfg(stageleft_runtime)]
hydro_lang::setup!();

use hydro_lang::prelude::*;

/// The smallest possible dataflow: add one to each input element.
pub fn add_one<'a, P>(input: Stream<u32, Process<'a, P>>) -> Stream<u32, Process<'a, P>> {
    input.map(q!(|x| x + 1))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Passes when run as `cargo test -p sim_repro_staged`.
    ///
    /// Exhibits UB (SIGSEGV on macOS/aarch64) when run as
    /// `cargo test --workspace`, because the sibling crate's
    /// `tokio = ["full"]` unifies into the host test binary while the sim
    /// dylib is still built with the minimal tokio feature set.
    #[test]
    fn add_one_adds_one() {
        let mut flow = FlowBuilder::new();
        let process = flow.process::<()>();

        let (in_port, input) = process.sim_input();
        let out_port = add_one(input).sim_output();

        flow.sim().exhaustive(async || {
            in_port.send(1);
            in_port.send(41);
            out_port.assert_yields_only([2, 42]).await;
        });
    }
}
