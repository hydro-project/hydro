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
#[expect(missing_docs, reason = "TODO")]
pub mod deploy_provider;

#[expect(missing_docs, reason = "TODO")]
pub mod builder;

#[cfg(stageleft_runtime)]
#[cfg(feature = "trybuild")]
#[cfg_attr(docsrs, doc(cfg(feature = "trybuild")))]
#[expect(missing_docs, reason = "TODO")]
#[cfg_attr(
    not(any(feature = "deploy", feature = "sim")),
    expect(
        dead_code,
        reason = "\"trybuild\" feature should be enabled by \"deploy\" and/or \"sim\""
    )
)]
pub mod trybuild;

#[cfg(stageleft_runtime)]
#[cfg(feature = "trybuild")]
#[cfg_attr(docsrs, doc(cfg(feature = "trybuild")))]
pub use trybuild::generate::init_test;
