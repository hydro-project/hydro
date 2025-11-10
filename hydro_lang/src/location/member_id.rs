use std::fmt::{Debug, Display};
use std::hash::Hash;
use std::marker::PhantomData;

use serde::{Deserialize, Serialize};

// #[repr(transparent)]
// pub struct MemberId<Tag> {
//     pub raw_id: u32,
//     pub(crate) _phantom: PhantomData<Tag>,
// }

// impl<Tag> MemberId<Tag> {
//     pub fn from_raw(id: u32) -> Self {
//         MemberId {
//             raw_id: id,
//             _phantom: PhantomData,
//         }
//     }
// }

// impl<Tag> Debug for MemberId<Tag> {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         write!(
//             f,
//             "MemberId::<{}>({})",
//             std::any::type_name::<Tag>(),
//             self.raw_id
//         )
//     }
// }

// impl<Tag> Display for MemberId<Tag> {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         write!(
//             f,
//             "MemberId::<{}>({})",
//             std::any::type_name::<Tag>(),
//             self.raw_id
//         )
//     }
// }

// impl<Tag> Clone for MemberId<Tag> {
//     fn clone(&self) -> Self {
//         *self
//     }
// }

// impl<Tag> Copy for MemberId<Tag> {}

// impl<Tag> Serialize for MemberId<Tag> {
//     fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
//     where
//         S: serde::ser::Serializer,
//     {
//         self.raw_id.serialize(serializer)
//     }
// }

// impl<'de, Tag> Deserialize<'de> for MemberId<Tag> {
//     fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
//     where
//         D: serde::de::Deserializer<'de>,
//     {
//         u32::deserialize(deserializer).map(|id| MemberId {
//             raw_id: id,
//             _phantom: PhantomData,
//         })
//     }
// }

// impl<Tag> PartialEq for MemberId<Tag> {
//     fn eq(&self, other: &Self) -> bool {
//         self.raw_id == other.raw_id
//     }
// }

// impl<Tag> Eq for MemberId<Tag> {}

// impl<Tag> Hash for MemberId<Tag> {
//     fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
//         self.raw_id.hash(state)
//     }
// }

#[derive(Deserialize, Serialize)]
pub enum MemberId<Tag> {
    Regular {
        raw_id: u32,
        _phantom: PhantomData<Tag>,
    },
    Docker {
        container_name: String,
        _phantom: PhantomData<Tag>,
    },
}

impl<Tag> MemberId<Tag> {
    pub fn from_raw(raw_id: u32) -> Self {
        MemberId::Regular {
            raw_id,
            _phantom: PhantomData,
        }
    }

    pub fn from_container_name(container_name: String) -> Self {
        MemberId::Docker {
            container_name,
            _phantom: PhantomData,
        }
    }

    pub fn get_raw_id(&self) -> u32 {
        match self {
            MemberId::Regular { raw_id, .. } => *raw_id,
            _ => panic!(),
        }
    }

    pub fn get_container_name(&self) -> String {
        match self {
            MemberId::Docker { container_name, .. } => container_name.clone(),
            _ => panic!(),
        }
    }

    pub fn into_tagless(self) -> MemberId<()> {
        match self {
            MemberId::Regular { raw_id, .. } => MemberId::Regular {
                raw_id,
                _phantom: PhantomData,
            },
            MemberId::Docker { container_name, .. } => MemberId::Docker {
                container_name,
                _phantom: PhantomData,
            },
        }
    }

    pub fn from_tagless(other: MemberId<()>) -> Self {
        match other {
            MemberId::Regular { raw_id, .. } => MemberId::Regular {
                raw_id,
                _phantom: PhantomData,
            },
            MemberId::Docker { container_name, .. } => MemberId::Docker {
                container_name,
                _phantom: PhantomData,
            },
        }
    }
}

impl<Tag> Debug for MemberId<Tag> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MemberId::Regular { raw_id, .. } => write!(
                f,
                "MemberId::<{}>({})",
                std::any::type_name::<Tag>(),
                raw_id
            ),
            MemberId::Docker { container_name, .. } => write!(
                f,
                "MemberId::<{}>(\"{}\")",
                std::any::type_name::<Tag>(),
                container_name
            ),
        }
    }
}

impl<Tag> Display for MemberId<Tag> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MemberId::Regular { raw_id, .. } => {
                write!(
                    f,
                    "MemberId::<{}>({})[Regular]",
                    std::any::type_name::<Tag>(),
                    raw_id
                )
            }
            MemberId::Docker { container_name, .. } => {
                write!(
                    f,
                    "MemberId::<{}>(\"{}\")[Docker]",
                    std::any::type_name::<Tag>(),
                    container_name
                )
            }
        }
    }
}

impl<Tag> PartialOrd for MemberId<Tag> {
    #[expect(
        clippy::non_canonical_partial_ord_impl,
        reason = "The implementation _is_ non-canonical."
    )]
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match (self, other) {
            (
                MemberId::Regular { raw_id, _phantom },
                MemberId::Regular {
                    raw_id: other_raw_id,
                    _phantom: _other_phantom,
                },
            ) => Some(raw_id.cmp(other_raw_id)),
            (
                MemberId::Docker {
                    container_name,
                    _phantom,
                },
                MemberId::Docker {
                    container_name: other_container_name,
                    _phantom: _other_phantom,
                },
            ) => Some(container_name.cmp(other_container_name)),
            _ => None,
        }
    }
}

impl<Tag> Ord for MemberId<Tag> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other)
            .expect("Can't compare different kinds of member ids")
    }
}

impl<Tag> Clone for MemberId<Tag> {
    fn clone(&self) -> Self {
        match self {
            MemberId::Regular { raw_id, .. } => MemberId::Regular {
                raw_id: *raw_id,
                _phantom: PhantomData,
            },
            MemberId::Docker { container_name, .. } => MemberId::Docker {
                container_name: container_name.clone(),
                _phantom: PhantomData,
            },
        }
    }
}

impl<Tag> PartialEq for MemberId<Tag> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (
                MemberId::Regular { raw_id, _phantom },
                MemberId::Regular {
                    raw_id: other_raw_id,
                    _phantom: _other_phantom,
                },
            ) => raw_id == other_raw_id,
            (
                MemberId::Docker {
                    container_name,
                    _phantom,
                },
                MemberId::Docker {
                    container_name: other_container_name,
                    _phantom: _other_phantom,
                },
            ) => container_name == other_container_name,
            _ => false,
        }
    }
}

impl<Tag> Eq for MemberId<Tag> {}

impl<Tag> Hash for MemberId<Tag> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            MemberId::Regular { raw_id, _phantom } => raw_id.hash(state),
            MemberId::Docker {
                container_name,
                _phantom,
            } => container_name.hash(state),
        }
    }
}
