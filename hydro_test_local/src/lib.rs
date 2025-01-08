stageleft::stageleft_crate!(hydro_test_local_macro);

#[cfg(stageleft_macro)]
pub(crate) mod local;
#[cfg(not(stageleft_macro))]
pub mod local;
