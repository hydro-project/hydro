stageleft::stageleft_no_entry_crate!();

pub mod debug;
pub mod decoupler;
pub mod deploy;
pub mod parse_results;
pub mod partition_node_analysis;
pub mod partition_syn_analysis;
pub mod partitioner;
pub mod repair;
pub mod rewrites;

#[cfg(feature = "ilp")]
#[cfg_attr(docsrs, doc(cfg(feature = "ilp")))]
pub mod decouple_analysis;
#[cfg(feature = "ilp")]
#[cfg_attr(docsrs, doc(cfg(feature = "ilp")))]
pub mod deploy_and_analyze;

#[cfg(test)]
mod test_init {
    #[ctor::ctor]
    fn init() {
        hydro_lang::deploy::init_test();
    }
}
