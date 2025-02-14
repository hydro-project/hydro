stageleft::stageleft_no_entry_crate!();

pub mod first_ten;
pub mod first_ten_cluster;
pub mod first_ten_distributed;

#[cfg(test)]
mod test_init {
    #[ctor::ctor]
    fn init() {
        hydro_lang::deploy::init_test();
    }
}
