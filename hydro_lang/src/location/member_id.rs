//! Typed and untyped identifiers for members of a [`Cluster`](super::Cluster).
//!
//! In Hydro, a [`Cluster`](super::Cluster) is a location that represents a group of
//! identical processes. Each individual process within a cluster is identified by a
//! [`MemberId`], which is parameterized by a tag type `Tag` to prevent accidentally
//! mixing up member IDs from different clusters.
//!
//! [`TaglessMemberId`] is the underlying untyped representation, which carries the
//! actual runtime identity (e.g. a raw numeric ID, a Docker container name, or a
//! Maelstrom node ID) without any compile-time cluster tag.

use std::fmt::{Debug, Display};
use std::hash::Hash;
use std::marker::PhantomData;

use serde::{Deserialize, Serialize};

/// An untyped identifier for a member of a cluster, without a compile-time tag
/// distinguishing which cluster it belongs to.
///
/// The available variants depend on which runtime features are enabled. This enum
/// is `#[non_exhaustive]` because new runtime backends may add additional variants.
///
/// In most user code, prefer [`MemberId<Tag>`] which carries a type-level tag to
/// prevent mixing up members from different clusters.
#[derive(Clone, Deserialize, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[non_exhaustive] // Variants change based on features.
pub enum TaglessMemberId {
    /// A legacy numeric member ID, used with the `deploy_integration` feature.
    #[cfg(feature = "deploy_integration")]
    #[cfg_attr(docsrs, doc(cfg(feature = "deploy_integration")))]
    Legacy {
        /// The raw numeric identifier for this cluster member.
        raw_id: u32,
    },
    /// A Docker container-based member ID, used with the `docker_runtime` feature.
    #[cfg(feature = "docker_runtime")]
    #[cfg_attr(docsrs, doc(cfg(feature = "docker_runtime")))]
    Docker {
        /// The Docker container name identifying this cluster member.
        container_name: String,
    },
    /// A Maelstrom node-based member ID, used with the `maelstrom_runtime` feature.
    #[cfg(feature = "maelstrom_runtime")]
    #[cfg_attr(docsrs, doc(cfg(feature = "maelstrom_runtime")))]
    Maelstrom {
        /// The Maelstrom node ID string identifying this cluster member.
        node_id: String,
    },
}

macro_rules! assert_feature {
    (#[cfg(feature = $feat:expr)] $( $code:stmt )+) => {
        #[cfg(not(feature = $feat))]
        panic!("Feature {:?} is not enabled.", $feat);

        #[cfg(feature = $feat)]
        {
            $( $code )+
        }
    };
}

impl TaglessMemberId {
    /// Creates a [`TaglessMemberId`] from a raw numeric ID.
    ///
    /// # Panics
    /// Panics if the `deploy_integration` feature is not enabled.
    pub fn from_raw_id(_raw_id: u32) -> Self {
        assert_feature! {
            #[cfg(feature = "deploy_integration")]
            Self::Legacy { raw_id: _raw_id }
        }
    }

    /// Returns the raw numeric ID from this member identifier.
    ///
    /// # Panics
    /// Panics if this is not the `Legacy` variant or if the `deploy_integration`
    /// feature is not enabled.
    pub fn get_raw_id(&self) -> u32 {
        assert_feature! {
            #[cfg(feature = "deploy_integration")]
            #[expect(clippy::allow_attributes, reason = "Depends on features.")]
            #[allow(
                irrefutable_let_patterns,
                reason = "Depends on features."
            )]
            let TaglessMemberId::Legacy { raw_id } = self else {
                panic!("Not `Legacy` variant.");
            }
            *raw_id
        }
    }

    /// Creates a [`TaglessMemberId`] from a Docker container name.
    ///
    /// # Panics
    /// Panics if the `docker_runtime` feature is not enabled.
    pub fn from_container_name(_container_name: impl Into<String>) -> Self {
        assert_feature! {
            #[cfg(feature = "docker_runtime")]
            Self::Docker {
                container_name: _container_name.into(),
            }
        }
    }

    /// Returns the Docker container name from this member identifier.
    ///
    /// # Panics
    /// Panics if this is not the `Docker` variant or if the `docker_runtime`
    /// feature is not enabled.
    pub fn get_container_name(&self) -> &str {
        assert_feature! {
            #[cfg(feature = "docker_runtime")]
            #[expect(clippy::allow_attributes, reason = "Depends on features.")]
            #[allow(
                irrefutable_let_patterns,
                reason = "Depends on features."
            )]
            let TaglessMemberId::Docker { container_name } = self else {
                panic!("Not `Docker` variant.");
            }
            container_name
        }
    }

    /// Creates a [`TaglessMemberId`] from a Maelstrom node ID.
    ///
    /// # Panics
    /// Panics if the `maelstrom_runtime` feature is not enabled.
    pub fn from_maelstrom_node_id(_node_id: impl Into<String>) -> Self {
        assert_feature! {
                #[cfg(feature = "maelstrom_runtime")]
                Self::Maelstrom {
                node_id: _node_id.into(),
            }
        }
    }

    /// Returns the Maelstrom node ID from this member identifier.
    ///
    /// # Panics
    /// Panics if this is not the `Maelstrom` variant or if the `maelstrom_runtime`
    /// feature is not enabled.
    pub fn get_maelstrom_node_id(&self) -> &str {
        assert_feature! {
            #[cfg(feature = "maelstrom_runtime")]
            #[expect(clippy::allow_attributes, reason = "Depends on features.")]
            #[allow(
                irrefutable_let_patterns,
                reason = "Depends on features."
            )]
            let TaglessMemberId::Maelstrom { node_id } = self else {
                panic!("Not `Maelstrom` variant.");
            }
            node_id
        }
    }
}

impl Display for TaglessMemberId {
    fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            #[cfg(feature = "deploy_integration")]
            TaglessMemberId::Legacy { raw_id } => write!(_f, "{:?}", raw_id),
            #[cfg(feature = "docker_runtime")]
            TaglessMemberId::Docker { container_name } => write!(_f, "{:?}", container_name),
            #[cfg(feature = "maelstrom_runtime")]
            TaglessMemberId::Maelstrom { node_id } => write!(_f, "{:?}", node_id),
            #[expect(
                clippy::allow_attributes,
                reason = "Only triggers when `TaglessMemberId` is empty."
            )]
            #[allow(
                unreachable_patterns,
                reason = "Needed when `TaglessMemberId` is empty."
            )]
            _ => panic!(),
        }
    }
}

/// A typed identifier for a member of a [`Cluster`](super::Cluster).
///
/// The `Tag` type parameter ties this ID to a specific cluster, preventing
/// accidental mixing of member IDs from different clusters at compile time.
/// Under the hood, this wraps a [`TaglessMemberId`].
#[repr(transparent)]
pub struct MemberId<Tag> {
    inner: TaglessMemberId,
    _phantom: PhantomData<Tag>,
}

impl<Tag> MemberId<Tag> {
    /// Converts this typed member ID into an untyped [`TaglessMemberId`],
    /// discarding the compile-time cluster tag.
    pub fn into_tagless(self) -> TaglessMemberId {
        self.inner
    }

    /// Creates a typed [`MemberId`] from an untyped [`TaglessMemberId`].
    pub fn from_tagless(inner: TaglessMemberId) -> Self {
        Self {
            inner,
            _phantom: Default::default(),
        }
    }

    /// Creates a typed [`MemberId`] from a raw numeric ID.
    ///
    /// # Panics
    /// Panics if the `deploy_integration` feature is not enabled.
    pub fn from_raw_id(raw_id: u32) -> Self {
        #[expect(clippy::allow_attributes, reason = "Depends on features.")]
        #[allow(
            unreachable_code,
            reason = "`inner` may be uninhabited depending on features."
        )]
        Self {
            inner: TaglessMemberId::from_raw_id(raw_id),
            _phantom: Default::default(),
        }
    }

    /// Returns the raw numeric ID from this member identifier.
    ///
    /// # Panics
    /// Panics if the underlying [`TaglessMemberId`] is not the `Legacy` variant
    /// or if the `deploy_integration` feature is not enabled.
    pub fn get_raw_id(&self) -> u32 {
        self.inner.get_raw_id()
    }
}

impl<Tag> Debug for MemberId<Tag> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self, f)
    }
}

impl<Tag> Display for MemberId<Tag> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "MemberId::<{}>({})",
            std::any::type_name::<Tag>(),
            self.inner
        )
    }
}

impl<Tag> Clone for MemberId<Tag> {
    fn clone(&self) -> Self {
        #[expect(clippy::allow_attributes, reason = "Depends on features.")]
        #[allow(
            unreachable_code,
            reason = "`inner` may be uninhabited depending on features."
        )]
        Self {
            inner: self.inner.clone(),
            _phantom: Default::default(),
        }
    }
}

impl<Tag> Serialize for MemberId<Tag> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.inner.serialize(serializer)
    }
}

impl<'a, Tag> Deserialize<'a> for MemberId<Tag> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'a>,
    {
        #[expect(clippy::allow_attributes, reason = "Depends on features.")]
        #[allow(
            unreachable_code,
            reason = "`inner` may be uninhabited depending on features."
        )]
        Ok(Self::from_tagless(TaglessMemberId::deserialize(
            deserializer,
        )?))
    }
}

impl<Tag> PartialOrd for MemberId<Tag> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<Tag> Ord for MemberId<Tag> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.inner.cmp(&other.inner)
    }
}

impl<Tag> PartialEq for MemberId<Tag> {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl<Tag> Eq for MemberId<Tag> {}

impl<Tag> Hash for MemberId<Tag> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.inner.hash(state);
        // This seems like the a good thing to do. This will ensure that two member ids that come from different
        // clusters but the same underlying host receive different hashes.
        std::any::type_name::<Tag>().hash(state);
    }
}
