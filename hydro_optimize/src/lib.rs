stageleft::stageleft_no_entry_crate!();

pub use stageleft::q;

// #[cfg(stageleft_runtime)]
pub mod inject_profiling;
pub mod parse_results;
pub mod repair;
pub mod deploy;
pub mod debug;
pub mod rewrites;
pub mod decouple_analysis;
pub mod decoupler;
pub mod partitioner;