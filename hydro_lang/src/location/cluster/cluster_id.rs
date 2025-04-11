use std::fmt::{Debug, Display};
use std::hash::Hash;
use std::marker::PhantomData;

use serde::{Deserialize, Serialize};

#[repr(transparent)]
pub struct ClusterId<ClusterTag> {
    pub raw_id: u32,
    pub(crate) _phantom: PhantomData<ClusterTag>,
}

impl<C> Debug for ClusterId<C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ClusterId::<{}>({})",
            std::any::type_name::<C>(),
            self.raw_id
        )
    }
}

impl<C> Display for ClusterId<C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ClusterId::<{}>({})",
            std::any::type_name::<C>(),
            self.raw_id
        )
    }
}

impl<C> Clone for ClusterId<C> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<C> Copy for ClusterId<C> {}

impl<C> Serialize for ClusterId<C> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        self.raw_id.serialize(serializer)
    }
}

impl<'de, C> Deserialize<'de> for ClusterId<C> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        u32::deserialize(deserializer).map(|id| ClusterId {
            raw_id: id,
            _phantom: PhantomData,
        })
    }
}

impl<C> PartialEq for ClusterId<C> {
    fn eq(&self, other: &Self) -> bool {
        self.raw_id == other.raw_id
    }
}

impl<C> Eq for ClusterId<C> {}

impl<C> Hash for ClusterId<C> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.raw_id.hash(state)
    }
}

impl<C> ClusterId<C> {
    pub fn from_raw(id: u32) -> Self {
        ClusterId {
            raw_id: id,
            _phantom: PhantomData,
        }
    }
}
