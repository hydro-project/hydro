//! Hydro compilation: the Hydro IR, Hydro to DFIR translation, and traits for deployment targets.

#[expect(missing_docs, reason = "TODO")]
pub mod ir;

#[cfg(feature = "build")]
#[cfg_attr(docsrs, doc(cfg(feature = "build")))]
#[expect(missing_docs, reason = "TODO")]
pub mod rewrites;

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
