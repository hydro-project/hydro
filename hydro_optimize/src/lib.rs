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
mod tests;

#[doc(hidden)]
#[cfg(doctest)]
mod docs {
    include_mdtests::include_mdtests!("hydro_optimize/docs/**/*.md*");
}

#[cfg(test)]
mod test_init {
    #[ctor::ctor]
    fn init() {
        hydro_lang::deploy::init_test();
    }
}
