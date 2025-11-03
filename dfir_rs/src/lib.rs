#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(missing_docs)]

//! DFIR is a low-level dataflow-based runtime system for the [Hydro Project](https://hydro.run/).
//!
//! The primary item in this crate is the [`Dfir`](crate::scheduled::graph::Dfir) struct,
//! representing a DFIR dataflow graph. Although this graph can be manually constructed, the
//! easiest way to instantiate a graph instance is with the [`dfir_syntax!`] macro using
//! DFIR's custom syntax.
//!
//! ```rust
//! let mut df = dfir_rs::dfir_syntax! {
//!     source_iter(["hello", "world"]) -> for_each(|s| println!("{}", s));
//! };
//! df.run_available();
//! ```
//!
//! For more examples, check out the [`examples` folder on Github](https://github.com/hydro-project/hydro/tree/main/dfir_rs/examples).

pub mod compiled;
pub mod scheduled;
pub mod util;

#[cfg(feature = "meta")]
#[cfg_attr(docsrs, doc(cfg(feature = "meta")))]
pub use dfir_lang as lang;
pub use variadics::{self, var_args, var_expr, var_type};
pub use {
    bincode, bytes, futures, itertools, lattices, pin_project_lite, rustc_hash, serde, serde_json,
    sinktools, tokio, tokio_stream, tokio_util, tracing, web_time,
};

/// `#[macro_use]` automagically brings the declarative macro export to the crate-level.
mod declarative_macro;
#[cfg_attr(docsrs, doc(cfg(feature = "dfir_macro")))]
#[cfg(feature = "dfir_macro")]
pub use dfir_macro::{
    DemuxEnum, dfir_main as main, dfir_parser, dfir_syntax, dfir_syntax_noemit, dfir_test as test,
    monotonic_fn, morphism,
};
pub use futures::never::Never;

#[cfg(doctest)]
mod booktest {
    mod surface_ops {
        include_mdtests::include_mdtests!("docs/docgen/*.md");
    }
}
