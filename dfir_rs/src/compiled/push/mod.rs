//! [`dfir_pipes::push::Push`]-based operator helpers.

#[cfg(feature = "dfir_macro")]
#[cfg_attr(docsrs, doc(cfg(feature = "dfir_macro")))]
mod demux_enum;
#[cfg(feature = "dfir_macro")]
#[cfg_attr(docsrs, doc(cfg(feature = "dfir_macro")))]
pub use demux_enum::DemuxEnum;
