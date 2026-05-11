//! DFIR syntax

#![warn(missing_docs)]
#![cfg_attr(nightly, feature(proc_macro_diagnostic, proc_macro_span))]

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
