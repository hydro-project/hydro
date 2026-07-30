#[cfg(stageleft_runtime)]
hydro_lang::setup!();

pub mod cluster;
pub mod distributed;
pub mod embedded;
pub mod external_client;
pub mod local;
#[cfg(feature = "tokio")]
pub mod maelstrom;
pub mod tutorials;

#[doc(hidden)]
#[cfg(doctest)]
mod docs {
    include_mdtests::include_mdtests!("docs/docs/hydro/**/*.md*");

    /// Registers a global constructor in the merged doctest binary so that `IS_TEST` mode is
    /// enabled for all doctests (required for doctests that compile Hydro programs, e.g. sim
    /// tests). This works because edition-2024 doctests are combined into a single binary, so
    /// the `ctor` below runs before every doctest in this crate.
    ///
    /// ```rust
    /// hydro_lang::macro_support::ctor::declarative::ctor!(
    ///     #[ctor(unsafe)]
    ///     fn init() {
    ///         hydro_lang::compile::init_test();
    ///     }
    /// );
    /// fn main() {}
    /// ```
    mod doctest_init {}
}
