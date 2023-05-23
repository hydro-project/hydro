#![feature(never_type)]
#![feature(type_alias_impl_trait)]
#![feature(impl_trait_in_assoc_type)]
#![allow(type_alias_bounds)]
#![allow(clippy::let_and_return)]
#![allow(clippy::iter_with_drain)]
#![allow(clippy::explicit_auto_deref)]
#![deny(missing_docs)] // TODO(mingwei): #![forbid(missing_docs)] when all docs are done.

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

pub use variadics::{self, var_args, var_expr, var_type};
pub use {
    bincode, bytes, futures, lattices, pusherator, rustc_hash, serde, serde_json, tokio,
    tokio_stream, tokio_util,
};

mod declarative_macro;
pub use declarative_macro::*;
#[cfg(feature = "hydroflow_datalog")]
pub use hydroflow_datalog::*;
#[cfg(feature = "hydroflow_macro")]
pub use hydroflow_macro::{
    hydroflow_main as main, hydroflow_parser, hydroflow_syntax, hydroflow_syntax_noemit,
    hydroflow_test as test,
};

#[cfg(doctest)]
mod booktest {
    macro_rules! booktest {
        ($path:literal, $i:ident) => {
            #[doc = include_str!(concat!("../../docs/docs/hydroflow/", $path, stringify!($i), ".md"))]
            mod $i {}
        };
    }

    booktest!("quickstart/", example_1_simplest);
    booktest!("quickstart/", example_2_simple);
    booktest!("quickstart/", example_3_stream);
    booktest!("quickstart/", example_4_neighbors);
    booktest!("quickstart/", example_5_reachability);
    booktest!("quickstart/", example_6_unreachability);
    booktest!("quickstart/", example_7_echo_server);
    booktest!("quickstart/", example_8_chat_server);

    booktest!("syntax/", index);
    booktest!("syntax/", surface_embedding);
    booktest!("syntax/", surface_flows);
    booktest!("syntax/", surface_data);

    mod surface_ops {
        hydroflow_macro::surface_booktest_operators!();
    }
}
