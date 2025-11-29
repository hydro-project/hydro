use std::fmt::{Debug, Display};
use std::hash::Hash;
use std::marker::PhantomData;

use serde::{Deserialize, Serialize};

// #[expect(
//     clippy::non_canonical_partial_ord_impl,
//     reason = "The implementation _is_ non-canonical."
// )]

#[derive(Clone, Hash, Deserialize, Serialize, Debug)]
pub enum TaglessMemberId {
    Legacy { raw_id: u32 },
    Docker { container_name: String },
}

impl TaglessMemberId {
    pub fn from_raw_id(raw_id: u32) -> Self {
        Self::Legacy { raw_id }
    }

    pub fn from_container_name(container_name: impl ToString) -> Self {
        Self::Docker {
            container_name: container_name.to_string(),
        }
    }

    pub fn get_raw_id(&self) -> u32 {
        match self {
            TaglessMemberId::Legacy { raw_id } => *raw_id,
            _ => panic!(),
        }
    }

    pub fn get_container_name(&self) -> String {
        match &self {
            TaglessMemberId::Docker { container_name } => container_name.clone(),
            _ => panic!(),
        }
    }
}

impl PartialEq for TaglessMemberId {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (
                TaglessMemberId::Legacy { raw_id },
                TaglessMemberId::Legacy {
                    raw_id: other_raw_id,
                },
            ) => raw_id == other_raw_id,
            (
                TaglessMemberId::Docker { container_name },
                TaglessMemberId::Docker {
                    container_name: other_container_name,
                },
            ) => container_name == other_container_name,
            _ => unreachable!(),
        }
    }
}

impl Eq for TaglessMemberId {}

// Comparing tags of different deployment origins means something has gone very wrong and the best thing to do is just crash immediately.
impl PartialOrd for TaglessMemberId {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match (self, other) {
            (
                TaglessMemberId::Legacy { raw_id },
                TaglessMemberId::Legacy {
                    raw_id: other_raw_id,
                },
            ) => raw_id.partial_cmp(other_raw_id),
            (
                TaglessMemberId::Docker { container_name },
                TaglessMemberId::Docker {
                    container_name: other_container_name,
                },
            ) => container_name.partial_cmp(other_container_name),
            _ => unreachable!(),
        }
    }
}

impl Ord for TaglessMemberId {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap()
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

    pub fn from_raw_id(raw_id: u32) -> Self {
        Self {
            inner: TaglessMemberId::from_raw_id(raw_id),
            _phantom: Default::default(),
        }
    }

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
        match &self.inner {
            TaglessMemberId::Legacy { raw_id, .. } => {
                write!(
                    f,
                    "MemberId::<{}>({})",
                    std::any::type_name::<Tag>(),
                    raw_id
                )
            }
            TaglessMemberId::Docker { container_name, .. } => {
                write!(
                    f,
                    "MemberId::<{}>(\"{}\")",
                    std::any::type_name::<Tag>(),
                    container_name
                )
            }
        }
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
        self.inner.partial_cmp(&other.inner)
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
        std::any::type_name::<Tag>().hash(state); // This seems like the a good thing to do. This will ensure that two member ids that come from different clusters but the same underlying host receive different hashes.
    }
}
