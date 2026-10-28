//! Hydro compilation: the Hydro IR, Hydro to DFIR translation, and traits for deployment targets.

#[expect(missing_docs, reason = "TODO")]
pub mod ir;

#[cfg(feature = "build")]
#[cfg_attr(docsrs, doc(cfg(feature = "build")))]
#[expect(missing_docs, reason = "TODO")]
pub mod built;

#[cfg(feature = "build")]
#[cfg_attr(docsrs, doc(cfg(feature = "build")))]
#[expect(missing_docs, reason = "TODO")]
pub mod compiled;

#[cfg(feature = "build")]
#[cfg_attr(docsrs, doc(cfg(feature = "build")))]
#[expect(missing_docs, reason = "TODO")]
pub mod deploy;

#[cfg(feature = "build")]
#[cfg_attr(docsrs, doc(cfg(feature = "build")))]
pub mod embedded;

pub mod embedded_runtime;

#[cfg(feature = "build")]
#[cfg_attr(docsrs, doc(cfg(feature = "build")))]
#[expect(missing_docs, reason = "TODO")]
pub mod deploy_provider;

#[expect(missing_docs, reason = "TODO")]
pub mod builder;

#[cfg(stageleft_runtime)]
#[cfg(feature = "trybuild")]
#[cfg_attr(docsrs, doc(cfg(feature = "trybuild")))]
#[expect(missing_docs, reason = "TODO")]
#[cfg_attr(
    not(any(feature = "deploy", feature = "sim", feature = "maelstrom")),
    expect(
        dead_code,
        reason = "\"trybuild\" feature should be enabled by \"deploy\" / \"sim\" / \"maelstrom\""
    )
)]
pub mod trybuild;

#[cfg(stageleft_runtime)]
#[cfg(feature = "trybuild")]
#[cfg_attr(docsrs, doc(cfg(feature = "trybuild")))]
pub use trybuild::generate::init_test;

/// Ident used for the DFIR runtime instance variable name.
#[cfg(feature = "build")]
#[cfg_attr(docsrs, doc(cfg(feature = "build")))]
pub(crate) const DFIR_IDENT: &str = "flow";

/// Returns the `dfir_rs::scheduled::metrics::MetricsState` typestate type that the DFIR codegen
/// will use for the generated `Dfir`'s `Context`, based on
/// [`AsCodeOptions::include_metrics_tracking`](dfir_lang::graph::AsCodeOptions::include_metrics_tracking).
///
/// This must match the type chosen by `DfirGraph::as_code_with_options`, and is used to reveal the
/// `Metrics` associated type in generated `Dfir<impl TickClosure<Metrics = ...>>` signatures so
/// callers can access `Dfir::metrics()` when (and only when) tracking is enabled.
#[cfg(feature = "build")]
pub(crate) fn dfir_metrics_state_type(
    as_code_options: &dfir_lang::graph::AsCodeOptions,
    root: &proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    if as_code_options.include_metrics_tracking {
        quote::quote! { #root::runtime_support::dfir_rs::scheduled::metrics::DfirMetrics }
    } else {
        quote::quote! { () }
    }
}
