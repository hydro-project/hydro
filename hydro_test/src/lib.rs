stageleft::stageleft_no_entry_crate!();

pub mod cluster;
pub mod distributed;
pub mod local;

#[doc(hidden)]
#[cfg(doctest)]
mod docs {
    include_mdtests::include_mdtests!("docs/docs/hydro/**/*.md*");
}

#[cfg(test)]
mod test_init {
    #[ctor::ctor]
    fn init() {
        hydro_lang::deploy::init_test();
    }
}
