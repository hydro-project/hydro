//! Push-based operator helpers, i.e. [`futures::sink::Sink`] helpers.

mod persist;
mod resolve_futures;
pub use persist::Persist;
pub use resolve_futures::ResolveFutures;

#[cfg(feature = "dfir_macro")]
#[cfg_attr(docsrs, doc(cfg(feature = "dfir_macro")))]
mod demux_enum;
#[cfg(feature = "dfir_macro")]
#[cfg_attr(docsrs, doc(cfg(feature = "dfir_macro")))]
pub use demux_enum::DemuxEnum;
