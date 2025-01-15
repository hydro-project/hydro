stageleft::stageleft_no_entry_crate!();

pub mod cluster;
pub mod distributed;

#[doc(hidden)]
#[stageleft::runtime]
#[cfg(doctest)]
mod docs {
    dfir_macro::doctest_markdown_glob!("docs/docs/hydro/**/*.md*");
}

#[stageleft::runtime]
#[cfg(test)]
mod tests {
    #[ctor::ctor]
    fn init() {
        hydro_lang::deploy::init_test();
    }
}
