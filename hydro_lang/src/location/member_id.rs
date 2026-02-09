use std::fmt::{Debug, Display};
use std::hash::Hash;
use std::marker::PhantomData;

use serde::{Deserialize, Serialize};

#[derive(Clone, Deserialize, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub(crate) enum TaglessMemberId {
    #[cfg(feature = "deploy")]
    #[cfg_attr(docsrs, doc(cfg(feature = "deploy")))]
    Legacy { raw_id: u32 },
    #[cfg(feature = "docker_deploy")]
    #[cfg_attr(docsrs, doc(cfg(feature = "docker_deploy")))]
    Docker { container_name: String },
    #[cfg(feature = "maelstrom")]
    #[cfg_attr(docsrs, doc(cfg(feature = "maelstrom")))]
    Maelstrom { node_id: String },
}

#[cfg(feature = "deploy")]
#[cfg_attr(docsrs, doc(cfg(feature = "deploy")))]
impl TaglessMemberId {
    pub fn from_raw_id(raw_id: u32) -> Self {
        Self::Legacy { raw_id }
    }

    pub fn get_raw_id(&self) -> u32 {
        let TaglessMemberId::Legacy { raw_id } = self else {
            panic!()
        };
        *raw_id
    }
}

#[cfg(feature = "docker_deploy")]
#[cfg_attr(docsrs, doc(cfg(feature = "docker_deploy")))]
impl TaglessMemberId {
    pub fn from_container_name(container_name: impl Into<String>) -> Self {
        Self::Docker {
            container_name: container_name.into(),
        }
    }

    pub fn get_container_name(&self) -> &str {
        let TaglessMemberId::Docker { container_name } = self else {
            panic!()
        };
        container_name
    }
}

#[cfg(feature = "maelstrom")]
#[cfg_attr(docsrs, doc(cfg(feature = "maelstrom")))]
impl TaglessMemberId {
    pub fn from_maelstrom_node_id(node_id: impl Into<String>) -> Self {
        Self::Maelstrom {
            node_id: node_id.into(),
        }
    }

    pub fn get_maelstrom_node_id(&self) -> &str {
        let TaglessMemberId::Maelstrom { node_id } = self else {
            panic!()
        };
        node_id
    }
}

impl Display for TaglessMemberId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            #[cfg(feature = "deploy")]
            TaglessMemberId::Legacy { raw_id } => write!(f, "{:?}", raw_id),
            #[cfg(feature = "docker_deploy")]
            TaglessMemberId::Docker { container_name } => write!(f, "{:?}", container_name),
            #[cfg(feature = "maelstrom")]
            TaglessMemberId::Maelstrom { node_id } => write!(f, "{:?}", node_id),
            #[expect(clippy::allow_attributes, reason = "Only triggers when `TaglessMemberId` is empty.")]
            #[allow(unreachable_patterns, reason = "Needed when `TaglessMemberId` is empty.")]
            _ => panic!(),
        }
    }
}

#[repr(transparent)]
pub struct MemberId<Tag> {
    inner: TaglessMemberId,
    _phantom: PhantomData<Tag>,
}

impl<Tag> MemberId<Tag> {
    pub fn into_tagless(self) -> TaglessMemberId {
        self.inner
    }

    pub fn from_tagless(inner: TaglessMemberId) -> Self {
        Self {
            inner,
            _phantom: Default::default(),
        }
    }

    #[cfg(feature = "deploy")]
    #[cfg_attr(docsrs, doc(cfg(feature = "deploy")))]
    pub fn from_raw_id(raw_id: u32) -> Self {
        Self {
            inner: TaglessMemberId::from_raw_id(raw_id),
            _phantom: Default::default(),
        }
    }

    #[cfg(feature = "deploy")]
    #[cfg_attr(docsrs, doc(cfg(feature = "deploy")))]
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
