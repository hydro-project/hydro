//! Module for [`StateHandle`], part of the "state API".

use std::any::{Any, TypeId};
use std::marker::PhantomData;

use super::StateId;

/// A handle into a particular [`Dfir`](super::graph::Dfir) instance, referring to data
/// inserted by [`add_state`](super::graph::Dfir::add_state).
///
/// If you need to store state handles in a data structure see [`StateHandleErased`] which hides
/// the generic type parameter.
#[must_use]
#[derive(Debug, PartialEq, Eq, Hash)]
pub struct StateHandle<T> {
    pub(crate) state_id: StateId,
    pub(crate) _phantom: PhantomData<*mut T>,
}
impl<T> Copy for StateHandle<T> {}
impl<T> Clone for StateHandle<T> {
    fn clone(&self) -> Self {
        *self
    }
}

/// A state handle with the generic type parameter erased, allowing it to be stored in omogenous
/// data structures. The type is tracked internally as data via [`TypeId`].
///
/// Use [`StateHandleErased::from(state_handle)`](StateHandleErased::from) to create an instance
/// from a typed [`StateHandle<T>`].
///
/// Use [`StateHandle::<T>::try_from()`](StateHandle::try_from) to convert the `StateHandleErased`
/// back into a `StateHandle<T>` of the given type `T`. If `T` is the wrong type then the original
/// `StateHandleErased` will be returned as the `Err`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct StateHandleErased {
    state_id: StateId,
    type_id: TypeId,
}

/// See [`StateHandleErased`].
impl<T> TryFrom<StateHandleErased> for StateHandle<T>
where
    T: Any,
{
    type Error = StateHandleErased;

    fn try_from(value: StateHandleErased) -> Result<Self, Self::Error> {
        if TypeId::of::<T>() == value.type_id {
            Ok(Self {
                state_id: value.state_id,
                _phantom: PhantomData,
            })
        } else {
            Err(value)
        }
    }
}
/// See [`StateHandleErased`].
impl<T> From<StateHandle<T>> for StateHandleErased
where
    T: Any,
{
    fn from(value: StateHandle<T>) -> Self {
        Self {
            state_id: value.state_id,
            type_id: TypeId::of::<T>(),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_erasure() {
        let handle = StateHandle::<String> {
            state_id: StateId::from_raw(0),
            _phantom: PhantomData,
        };
        let handle_erased = StateHandleErased::from(handle);
        let handle_good = StateHandle::<String>::try_from(handle_erased);
        let handle_bad = StateHandle::<&'static str>::try_from(handle_erased);

        assert_eq!(Ok(handle), handle_good);
        assert_eq!(Err(handle_erased), handle_bad);
    }
}
