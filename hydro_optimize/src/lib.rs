// TODO(shadaj): this should be stageleft::stageleft_no_q_crate()
#[doc(hidden)]
#[cfg(test)]
pub mod __staged {
    include!(concat!(
        env!("OUT_DIR"),
        stageleft::PATH_SEPARATOR!(),
        "staged_deps.rs"
    ));
}

pub mod debug;
pub mod decouple_analysis;
pub mod decoupler;
pub mod deploy;
pub mod deploy_and_analyze;
pub mod parse_results;
pub mod partition_node_analysis;
pub mod partition_syn_analysis;
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
