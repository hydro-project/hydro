//! Push-based operator helpers, i.e. [`futures::sink::Sink`] helpers.

mod filter;
mod filter_map;
mod flat_map;
mod flatten;
mod for_each;
mod inspect;
mod map;
mod partition;
mod persist;
mod resolve_futures;
mod unzip;
pub use filter::Filter;
pub use filter_map::FilterMap;
pub use flat_map::FlatMap;
pub use flatten::Flatten;
pub use for_each::ForEach;
pub use inspect::Inspect;
pub use map::Map;
pub use partition::{Partition, SinkVariadic};
pub use persist::Persist;
pub use resolve_futures::ResolveFutures;
pub use unzip::Unzip;

#[cfg(feature = "dfir_macro")]
#[cfg_attr(docsrs, doc(cfg(feature = "dfir_macro")))]
mod demux_enum;
#[cfg(feature = "dfir_macro")]
#[cfg_attr(docsrs, doc(cfg(feature = "dfir_macro")))]
pub use demux_enum::DemuxEnum;
