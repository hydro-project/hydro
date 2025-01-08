stageleft::stageleft_no_entry_crate!();

pub mod cluster;
pub mod distributed;

#[doc(hidden)]
#[stageleft::runtime]
mod docs {
    #[doc = include_str!("../../docs/docs/hydro/consistency.md")]
    mod consistency {}
}
