#[derive(Copy, Clone)]
pub struct NonDet;

#[doc(inline)]
pub use crate::__nondet__ as nondet;

#[macro_export]
/// Fulfills a non-determinism guard parameter by declaring a reason why the
/// non-determinism is tolerated or providing other non-determinism guards
/// that forward the inner non-determinism.
///
/// The first argument must be a string literal with the reason the non-determinism
/// is okay. If forwarding a parent non-determinism, you should provide a short
/// explanation of how the inner non-determinism is captured by the outer one,
/// and also discuss any forms of the inner non-determinism that will not be exposed
/// outside if they are locally resolved
macro_rules! __nondet__ {
    ($(#[doc = $doc:expr])+$($forward:ident),*) => {
        {
            $(let _ = $forward;)*
            $crate::nondet::NonDet
        }
    };
}
