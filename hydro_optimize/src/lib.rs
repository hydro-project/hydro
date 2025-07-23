pub mod debug;
pub mod decouple_analysis;
pub mod decoupler;
pub mod deploy;
pub mod deploy_and_analyze;
pub mod parse_results;
pub mod partitioner;
pub mod repair;
pub mod rewrites;

#[cfg(test)]
mod test_init {
    #[ctor::ctor]
    fn init() {
        hydro_lang::deploy::init_test();
    }
}
