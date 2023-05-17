#![feature(never_type)]
#![feature(type_alias_impl_trait)]
#![feature(impl_trait_in_assoc_type)]
#![allow(type_alias_bounds)]
#![allow(clippy::let_and_return)]
#![allow(clippy::iter_with_drain)]
#![allow(clippy::explicit_auto_deref)]

pub mod compiled;
pub mod lang;
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
pub use hydroflow_macro::*;

#[cfg(doctest)]
mod booktest {
    macro_rules! booktest {
        ($i:ident $( $t:tt )*) => {
            #[doc = include_str!(concat!("../../book/", stringify!($i), $( stringify!($t), )* ".md"))]
            mod $i {}
        };
    }

    booktest!(example_1_simplest);
    booktest!(example_2_simple);
    booktest!(example_3_stream);
    booktest!(example_4_neighbors);
    booktest!(example_5_reachability);
    booktest!(example_6_unreachability);
    booktest!(example_7_echo_server);
    booktest!(example_8_chat_server);

    booktest!(surface_syntax);
    booktest!(surface_embedding);
    booktest!(surface_flows);
    booktest!(surface_data);

    mod surface_ops {
        hydroflow_macro::surface_booktest_operators!();
    }
}
