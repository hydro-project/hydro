stageleft::stageleft_no_entry_crate!();

pub mod cluster;
pub mod distributed;
pub mod external_client;
pub mod local;

#[doc(hidden)]
#[cfg(doctest)]
mod docs {
    include_mdtests::include_mdtests!("docs/docs/hydro/**/*.md*");
}

#[ctor::ctor]
fn init_rewrites() {
    stageleft::add_private_reexport(
        vec!["tokio_util", "codec", "lines_codec"],
        vec!["tokio_util", "codec"],
    );
}

#[cfg(test)]
mod test_init {
    #[ctor::ctor]
    fn init() {
        hydro_lang::deploy::init_test();
    }
}
