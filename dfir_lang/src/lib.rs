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
