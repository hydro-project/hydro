stageleft::stageleft_no_entry_crate!();

pub mod bench_client;
pub mod compartmentalize;
pub mod quorum;
pub mod request_response;

#[cfg(test)]
mod test_init {
    #[ctor::ctor]
    fn init() {
        hydro_lang::deploy::init_test();
    }
}
