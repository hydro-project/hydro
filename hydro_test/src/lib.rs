stageleft::stageleft_no_entry_crate!();

pub mod cluster;
pub mod distributed;

#[doc(hidden)]
#[stageleft::runtime]
mod docs {
    #[doc = include_str!("../../docs/docs/hydro/consistency.md")]
    mod consistency {}
}

#[stageleft::runtime]
#[cfg(test)]
mod tests {
    #[ctor::ctor]
    fn init() {
        hydro_lang::deploy::init_test();
    }
}