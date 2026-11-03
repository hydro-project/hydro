//! DFIR syntax
#![cfg_attr(not(feature = "std"), no_std)]
#![warn(missing_docs)]
#![cfg_attr(all(nightly, feature = "codegen"), feature(proc_macro_diagnostic))]

#[cfg(any(test, feature = "alloc"))]
extern crate alloc;
#[cfg(any(test, feature = "std"))]
extern crate std;

pub mod graph_ids;

#[cfg(feature = "codegen")]
pub mod diagnostic;
#[cfg(feature = "codegen")]
pub mod graph;
#[cfg(feature = "codegen")]
pub mod parse;
#[cfg(feature = "codegen")]
pub mod pretty_span;
#[cfg(feature = "codegen")]
pub mod process_singletons;
#[cfg(feature = "codegen")]
pub mod union_find;

/// Handles a `dest_sink` send failure from generated code.
///
/// A [`std::io::ErrorKind::BrokenPipe`] (including one wrapped via
/// [`std::error::Error::source`]) is logged and dropped so shutdown can close the
/// receiver before the sender. Any other error panics with its `Debug` text.
#[cfg(feature = "codegen")]
#[doc(hidden)]
pub fn handle_dest_sink_error<E>(err: E)
where
    E: std::error::Error + 'static,
{
    graph::ops::dest_sink::handle_dest_sink_error(err);
}
