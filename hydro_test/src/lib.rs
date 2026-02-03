#[cfg(stageleft_runtime)]
hydro_lang::setup!();

pub mod cluster;
pub mod distributed;
pub mod external_client;
pub mod local;
pub mod maelstrom;
pub mod tutorials;

#[doc(hidden)]
#[cfg(doctest)]
mod docs {
    include_mdtests::include_mdtests!("docs/docs/hydro/**/*.md*");
}
