//! Fixture: source of the `sim_repro_staged` crate scaffolded by the
//! `sim_feature_unification` meta-test (see `../sim_feature_unification.rs`).
//!
//! This file is not compiled as part of `hydro_lang`; it is written into a
//! generated cargo workspace at test time via `include_str!`.
//!
//! Minimal staged crate reproducing host/dylib dependency-feature divergence
//! in Hydro's simulator. The simulator (`flow.sim()`) compiles staged
//! dataflow into a `cdylib` via a synthetic cargo workspace under
//! `target/hydro_trybuild/<crate>/`, `dlopen`s it, and shares Rust data
//! structures across the boundary (e.g. the `tokio::sync::mpsc` unbounded
//! channels backing `sim_input`/`sim_output` ports are created inside the
//! dylib and polled by the host's `SimReceiver`). That is only sound if the
//! host test binary and the dylib are compiled with *identical* dependency
//! configurations. See the meta-test's module docs for the full mechanism.

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
    /// Exhibits UB (observed as a SIGSEGV on macOS/aarch64) when run as
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

    /// Deterministic, platform-independent demonstration of the host/dylib
    /// divergence — no reliance on the UB manifesting as a crash.
    ///
    /// The `q!()` closure runs *inside the sim dylib*, so its
    /// `size_of::<tokio::runtime::Handle>()` is evaluated against the tokio
    /// that the trybuild workspace resolved; the assertion below evaluates
    /// the same expression against the tokio linked into the *host* test
    /// binary. These are only equal if both sides were compiled with the
    /// same tokio configuration:
    ///
    /// - `cargo test -p sim_repro_staged`: both resolve the minimal feature
    ///   set -> `Handle` is 8 bytes on both sides -> passes.
    /// - `cargo test --workspace`: the sibling crate's `tokio/full` unifies
    ///   `rt-multi-thread` into the host (16-byte `Handle`, the runtime
    ///   scheduler enum gains a multi-thread variant) while the dylib stays
    ///   minimal (8 bytes) -> fails.
    ///
    /// `tokio::runtime::Handle` is just a convenient *witness*: with
    /// tokio v1.53.1, `rt-multi-thread` alone also changes
    /// `size_of` for `Runtime` (64 -> 80), `time::Sleep` (96 -> 112), and
    /// `net::TcpListener` (32 -> 40). Any such mismatch means the host and
    /// the dylib disagree about the layout of types they share across the
    /// `dlopen` boundary (e.g. the `mpsc::UnboundedReceiver<Bytes>` backing
    /// every sim port), which is undefined behavior regardless of whether it
    /// crashes on a given platform.
    #[test]
    fn host_and_dylib_agree_on_tokio_layout() {
        let mut flow = FlowBuilder::new();
        let process = flow.process::<()>();

        let (in_port, input) = process.sim_input();
        let out_port = input
            .map(q!(|_probe: ()| {
                std::mem::size_of::<tokio::runtime::Handle>() as u64
            }))
            .sim_output();

        let host_size = std::mem::size_of::<tokio::runtime::Handle>() as u64;
        flow.sim().exhaustive(async || {
            in_port.send(());
            let dylib_size = out_port.next().await;
            assert_eq!(
                host_size, dylib_size,
                "the host test binary and the sim dylib were compiled against \
                 differently-configured tokio crates \
                 (size_of::<tokio::runtime::Handle>: host = {host_size}, \
                 dylib = {dylib_size}); they share tokio channel internals \
                 across the dlopen boundary, so this divergence is undefined \
                 behavior. See create_trybuild() in \
                 hydro_lang/src/compile/trybuild/generate.rs and the \
                 sim_feature_unification meta-test in hydro_lang/tests."
            );
        });
    }
}
