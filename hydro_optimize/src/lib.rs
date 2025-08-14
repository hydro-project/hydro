#[doc(hidden)]
#[cfg(test)]
pub mod __staged {
    #[ctor::ctor]
    fn setup_hydro_lang_dep() {
        // `hydro_lang` is an "undeclared" dependency of `hydro_optimize`
        // That is, when we `quote_type`, we rely on some other crate with
        // `stageleft::stageleft_no_entry_crate!()` to define a path to `hydro_lang`.
        // In unit tests for `hydro_optimize`, this causes trouble because no such
        // crate exists, so we would emit a raw reference to `hydro_lang`, which might
        // not exist if there is aliasing. But `hydro_lang::deploy::init_test` causes
        // the generated binary to include its own `__deps` module, which will provide
        // the path `__staged::__deps::hydro_lang`, even if there is aliasing. So when
        // running tests we artificially make `quote_type` aware of this path.
        //
        // Eventually, we should either make `hydro_optimize` a "normal" crate
        // that also uses `stageleft::stageleft_no_entry_crate!()` or we should
        // properly handle undeclared dependencies in `quote_type` (by using
        // `proc_macro_crate` to handle aliasing).
        stageleft::internal::add_deps_reexport(
            vec!["hydro_lang"],
            vec!["hydro_optimize", "__staged", "__deps", "hydro_lang"],
        );
    }
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
