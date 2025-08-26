use std::fmt::{Debug, Display};
use std::hash::Hash;
use std::marker::PhantomData;

use serde::{Deserialize, Serialize};

#[repr(transparent)]
pub struct MemberId<Tag> {
    pub raw_id: u32,
    pub(crate) _phantom: PhantomData<Tag>,
}

impl<C> Debug for MemberId<C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "MemberId::<{}>({})",
            std::any::type_name::<C>(),
            self.raw_id
        )
    }
}

impl<C> Display for MemberId<C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "MemberId::<{}>({})",
            std::any::type_name::<C>(),
            self.raw_id
        )
    }
}

impl<C> Clone for MemberId<C> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<C> Copy for MemberId<C> {}

impl<C> Serialize for MemberId<C> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        self.raw_id.serialize(serializer)
    }
}

impl<'de, C> Deserialize<'de> for MemberId<C> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        u32::deserialize(deserializer).map(|id| MemberId {
            raw_id: id,
            _phantom: PhantomData,
        })
    }
}

impl<C> PartialEq for MemberId<C> {
    fn eq(&self, other: &Self) -> bool {
        self.raw_id == other.raw_id
    }
}

impl<C> Eq for MemberId<C> {}

impl<C> Hash for MemberId<C> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.raw_id.hash(state)
    }
}

impl<C> MemberId<C> {
    pub fn from_raw(id: u32) -> Self {
        MemberId {
            raw_id: id,
            _phantom: PhantomData,
        }
    }
}
