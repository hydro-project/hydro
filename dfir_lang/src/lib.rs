//! DFIR syntax

#![warn(missing_docs)]
#![cfg_attr(all(nightly, feature = "codegen"), feature(proc_macro_diagnostic))]

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
