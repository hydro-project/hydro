stageleft::stageleft_no_entry_crate!();

pub mod compartmentalize;
pub mod quorum;
pub mod request_response;

#[stageleft::runtime]
#[cfg(test)]
mod tests {
    #[ctor::ctor]
    fn init() {
        hydro_lang::deploy::init_test();
    }
}
