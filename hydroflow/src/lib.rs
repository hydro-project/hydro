#![cfg_attr(feature = "nightly", feature(never_type))]
#![allow(type_alias_bounds)]
#![allow(clippy::let_and_return)]
#![allow(clippy::iter_with_drain)]
#![allow(clippy::explicit_auto_deref)]
#![warn(missing_docs)]

//! Hydroflow is a low-level dataflow-based runtime system for the [Hydro Project](https://hydro.run/).
//!
//! The primary item in this crate is the [`Hydroflow`](crate::scheduled::graph::Hydroflow) struct,
//! representing a Hydroflow dataflow graph. Although this graph can be manually constructed, the
//! easiest way to instantiate a `Hydroflow` instance is with the [`hydroflow_syntax!`] macro using
//! Hydroflow's custom "surface syntax."
//!
//! ```rust
//! let mut hf = hydroflow::hydroflow_syntax! {
//!     source_iter(["hello", "world"]) -> for_each(|s| println!("{}", s));
//! };
//! hf.run_available();
//! ```
//!
//! For more examples, check out the [`examples` folder on Github](https://github.com/hydro-project/hydroflow/tree/main/hydroflow/examples).

pub mod compiled;
pub mod props;
pub mod scheduled;
pub mod util;

#[cfg(feature = "python")]
pub use pyo3;
pub use variadics::{self, var_args, var_expr, var_type};
pub use {
    bincode, bytes, futures, hydroflow_lang as lang, itertools, lattices, pusherator, rustc_hash,
    serde, serde_json, tokio, tokio_stream, tokio_util, tracing, web_time,
};

/// `#[macro_use]` automagically brings the declarative macro export to the crate-level.
mod declarative_macro;
#[cfg(feature = "hydroflow_datalog")]
pub use hydroflow_datalog::*;
#[cfg(feature = "hydroflow_macro")]
pub use hydroflow_macro::{
    hydroflow_main as main, hydroflow_parser, hydroflow_syntax, hydroflow_syntax_noemit,
    hydroflow_test as test, monotonic_fn, morphism, DemuxEnum,
};

/// Stand-in for the [nightly "never" type `!`](https://doc.rust-lang.org/std/primitive.never.html)
#[cfg(not(feature = "nightly"))]
pub type Never = std::convert::Infallible;
/// The [nightly "never" type `!`](https://doc.rust-lang.org/std/primitive.never.html)
#[cfg(feature = "nightly")]
pub type Never = !;

#[cfg(doctest)]
mod booktest {
    mod surface_ops {
        hydroflow_macro::surface_booktest_operators!();
    }
}
