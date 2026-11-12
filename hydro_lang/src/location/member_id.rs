//! Typed identifiers for members of a [`Cluster`](super::Cluster).
//!
//! In Hydro, a [`Cluster`](super::Cluster) is a location that represents a group of
//! identical processes. Each process in a cluster is identified by a [`MemberId`].
//!
//! A `MemberId<Tag, Id>` has two type parameters.
//! `Tag` names the member's cluster, so IDs from different clusters cannot be mixed up.
//! `Id` is the type of the ID itself, such as an index, an address, or a name.
//!
//! You choose `Id` when you create the cluster (see
//! [`FlowBuilder::cluster`](crate::compile::builder::FlowBuilder::cluster)).
//! Pick whatever the code that runs the cluster already uses to identify its members.
//! Hydro only compares, hashes, orders, clones, and sends IDs, so it works with any such type.
//! The deployment backend must support it (see
//! [`SupportsMemberId`](crate::compile::deploy_provider::SupportsMemberId)).

use std::fmt::{Debug, Display};
use std::hash::Hash;
use std::marker::PhantomData;

use serde::{Deserialize, Serialize};

/// A typed identifier for a member of a [`Cluster`](super::Cluster).
///
/// The `Tag` type parameter ties this ID to a specific cluster, preventing
/// accidental mixing of member IDs from different clusters at compile time.
/// It wraps an `Id` (see the [module documentation](self)).
#[repr(transparent)]
pub struct MemberId<Tag, Id = u32> {
    inner: Id,
    _phantom: PhantomData<Tag>,
}

impl<Tag, Id> MemberId<Tag, Id> {
    /// Unwraps the ID, dropping the cluster tag.
    pub fn into_tagless(self) -> Id {
        self.inner
    }

    /// Wraps an ID as a member of this cluster.
    pub fn from_tagless(inner: Id) -> Self {
        Self {
            inner,
            _phantom: PhantomData,
        }
    }

    /// The wrapped ID.
    pub fn as_tagless(&self) -> &Id {
        &self.inner
    }
}

impl<Tag> MemberId<Tag, u32> {
    /// The member numbered `raw_id`.
    ///
    /// Hydro Deploy and the simulator number each cluster's members `0..n`.
    pub fn from_raw_id(raw_id: u32) -> Self {
        Self::from_tagless(raw_id)
    }

    /// This member's number.
    pub fn get_raw_id(&self) -> u32 {
        self.inner
    }
}

impl<Tag, Id: Debug> Debug for MemberId<Tag, Id> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "MemberId::<{}>({:?})",
            std::any::type_name::<Tag>(),
            self.inner
        )
    }
}

impl<Tag, Id: Display> Display for MemberId<Tag, Id> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "MemberId::<{}>({})",
            std::any::type_name::<Tag>(),
            self.inner
        )
    }
}

impl<Tag, Id: Clone> Clone for MemberId<Tag, Id> {
    fn clone(&self) -> Self {
        Self::from_tagless(self.inner.clone())
    }
}

impl<Tag, Id: Serialize> Serialize for MemberId<Tag, Id> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.inner.serialize(serializer)
    }
}

impl<'a, Tag, Id: Deserialize<'a>> Deserialize<'a> for MemberId<Tag, Id> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'a>,
    {
        Ok(Self::from_tagless(Id::deserialize(deserializer)?))
    }
}

impl<Tag, Id: Ord> PartialOrd for MemberId<Tag, Id> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<Tag, Id: Ord> Ord for MemberId<Tag, Id> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.inner.cmp(&other.inner)
    }
}

impl<Tag, Id: PartialEq> PartialEq for MemberId<Tag, Id> {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl<Tag, Id: Eq> Eq for MemberId<Tag, Id> {}

impl<Tag, Id: Hash> Hash for MemberId<Tag, Id> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.inner.hash(state);
        // This seems like the a good thing to do. This will ensure that two member ids that come from different
        // clusters but the same underlying host receive different hashes.
        std::any::type_name::<Tag>().hash(state);
    }
}
