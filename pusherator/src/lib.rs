#![cfg_attr(docsrs, feature(doc_cfg))]
//! Combinators for working with push-based iterators.
//!
//! These are grouped into several modules:
//! * [`pusherator`] - push-equivalent to [`Iterator`]s, synchronous.
//! * ['sink`] - push-equivalent to [`Stream`](futures::stream::Stream)s, asynchronous, as provided by [`futures`].
//! * [`sinkerator`] - asynchronous hybrid of pusherators and sinks, lighter weight than the [`Sink`](futures::sink::Sink) interface.

#[cfg(feature = "futures")]
#[cfg_attr(docsrs, doc(cfg(feature = "futures")))]
pub use futures;
#[cfg(feature = "futures")]
#[cfg_attr(docsrs, doc(cfg(feature = "futures")))]
pub use futures::never::Never;
#[cfg(feature = "variadics")]
#[cfg_attr(docsrs, doc(cfg(feature = "variadics")))]
pub use variadics;

#[cfg(feature = "pusherator")]
#[cfg_attr(docsrs, doc(cfg(feature = "pusherator")))]
pub mod pusherator;
#[cfg(feature = "pusherator")]
#[cfg_attr(docsrs, doc(cfg(feature = "pusherator")))]
pub use pusherator::*;

#[cfg(feature = "sink")]
#[cfg_attr(docsrs, doc(cfg(feature = "sink")))]
pub mod sink;
#[cfg(feature = "sinkerator")]
#[cfg_attr(docsrs, doc(cfg(feature = "sinkerator")))]
pub mod sinkerator;
