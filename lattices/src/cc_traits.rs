//! Re-export of the `cc_traits` crate with [`SimpleKeyedRef`] added.

pub use ::cc_traits::*;

/// Keyed collection where each key reference can be converted into a standard
/// "simple" rust reference.
///
/// This trait is particularly useful to avoid having to include where bounds
/// of the form `for<'r> T::KeyRef<'r>: Into<&'r T::Key>`, which can
/// currently lead the compiler to try to prove `T: 'static`
/// (see <https://github.com/rust-lang/rust/pull/96709#issuecomment-1182403490>)
/// for more details.
///
/// <https://github.com/timothee-haudebourg/cc-traits/pull/8>
pub trait SimpleKeyedRef: KeyedRef {
    /// Convert the borrow into a simple `&Key` ref.
    fn into_ref<'r>(r: Self::KeyRef<'r>) -> &'r Self::Key
    where
        Self: 'r;
}
impl<T> SimpleKeyedRef for T
where
    T: KeyedRef,
    for<'a> Self::KeyRef<'a>: Into<&'a Self::Key>,
{
    fn into_ref<'r>(r: Self::KeyRef<'r>) -> &'r Self::Key
    where
        Self: 'r,
    {
        r.into()
    }
}
