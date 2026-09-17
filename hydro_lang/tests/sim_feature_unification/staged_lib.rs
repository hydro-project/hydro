//! Fixture: source of the `sim_repro_staged` crate scaffolded by the
//! `sim_feature_unification` meta-test (see `../sim_feature_unification.rs`).
//!
//! This file is not compiled as part of `hydro_lang`; it is written into a
//! generated cargo workspace at test time via `include_str!`.
//!
//! The meta-test runs this crate's tests twice: once alone
//! (`cargo test -p sim_repro_staged`), and once together with a sibling crate
//! (`cargo test --workspace`) whose only purpose is to feature-unify
//! `tokio/full` and `smallvec/union` into the host test binary. The sim dylib
//! is resolved from this crate's manifest alone, so under the second
//! invocation the host and the dylib are built against differently-configured
//! copies of those dependencies. See the meta-test's module docs for why that
//! must be harmless.

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

    /// Set by the meta-test for the `cargo test --workspace` invocation, where
    /// the sibling crate unifies extra features into the host build.
    const DIVERGENCE_ENV: &str = "SIM_REPRO_EXPECT_HOST_DYLIB_DIVERGENCE";

    /// Round-trips values through both sim ports (`sim_input` into the dylib,
    /// `sim_output` back out). This is the code path that shares channel
    /// state across the `dlopen` boundary, and it must work regardless of
    /// which dependency features the host build happens to unify.
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

    /// Guards the premise of the meta-test's workspace invocation: the host
    /// test binary and the sim dylib really are built against
    /// differently-configured dependencies there, so `add_one_adds_one`
    /// passing under that invocation is evidence that the boundary is
    /// insensitive to the divergence, not an accident of aligned feature sets.
    ///
    /// The `q!()` closure runs *inside the sim dylib*, so its
    /// `size_of::<SmallVec<[u64; 2]>>()` is evaluated against the `smallvec`
    /// that the generated trybuild workspace resolved (from this crate's
    /// manifest: no `union`); the host value is evaluated against the
    /// `smallvec` linked into the test binary (`union` when the sibling
    /// participates). smallvec's `union` feature exists solely to shrink this
    /// layout by one word, so the two values differ exactly when the feature
    /// sets do.
    #[test]
    fn smallvec_layout_diverges_only_under_workspace_unification() {
        let expect_divergence = std::env::var_os(DIVERGENCE_ENV).is_some();

        let mut flow = FlowBuilder::new();
        let process = flow.process::<()>();

        let (in_port, input) = process.sim_input();
        let out_port = input
            .map(q!(|_probe: ()| {
                std::mem::size_of::<smallvec::SmallVec<[u64; 2]>>() as u64
            }))
            .sim_output();

        let host_size = std::mem::size_of::<smallvec::SmallVec<[u64; 2]>>() as u64;
        flow.sim().exhaustive(async || {
            in_port.send(());
            let dylib_size = out_port.next().await;
            if expect_divergence {
                assert_ne!(
                    host_size, dylib_size,
                    "expected the sibling crate's `smallvec/union` to reach the host build but \
                     not the sim dylib (size_of::<SmallVec<[u64; 2]>>: host = {host_size}, \
                     dylib = {dylib_size}); the workspace invocation is no longer exercising \
                     host/dylib feature divergence"
                );
            } else {
                assert_eq!(
                    host_size, dylib_size,
                    "with no sibling crate in the invocation the host and the sim dylib should \
                     resolve identical `smallvec` features (size_of::<SmallVec<[u64; 2]>>: \
                     host = {host_size}, dylib = {dylib_size})"
                );
            }
        });
    }
}
