use std::fmt::{Debug, Display};
use std::hash::Hash;
use std::marker::PhantomData;

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub enum MemberId<Tag> {
    Legacy {
        raw_id: u32,
        _phantom: PhantomData<Tag>,
    },
}

impl<Tag> MemberId<Tag> {
    pub fn from_raw_id(raw_id: u32) -> Self {
        MemberId::Legacy {
            raw_id,
            _phantom: PhantomData,
        }
    }

    pub fn get_raw_id(&self) -> u32 {
        match self {
            MemberId::Legacy { raw_id, .. } => *raw_id,
        }
    }

    pub fn into_tagless(self) -> MemberId<()> {
        match self {
            MemberId::Legacy { raw_id, .. } => MemberId::Legacy {
                raw_id,
                _phantom: PhantomData,
            },
        }
    }

    pub fn from_tagless(other: MemberId<()>) -> Self {
        match other {
            MemberId::Legacy { raw_id, .. } => MemberId::Legacy {
                raw_id,
                _phantom: PhantomData,
            },
        }
    }
}

impl<Tag> Debug for MemberId<Tag> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MemberId::Legacy { raw_id, .. } => write!(
                f,
                "MemberId::<{}>({})",
                std::any::type_name::<Tag>(),
                raw_id
            ),
        }
    }
}

impl<Tag> Display for MemberId<Tag> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MemberId::Legacy { raw_id, .. } => {
                write!(
                    f,
                    "MemberId::<{}>({})",
                    std::any::type_name::<Tag>(),
                    raw_id
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
                MemberId::Legacy { raw_id, _phantom },
                MemberId::Legacy {
                    raw_id: other_raw_id,
                    _phantom: _other_phantom,
                },
            ) => Some(raw_id.cmp(other_raw_id)),
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
            MemberId::Legacy { raw_id, .. } => MemberId::Legacy {
                raw_id: *raw_id,
                _phantom: PhantomData,
            },
        }
    }
}

impl<Tag> PartialEq for MemberId<Tag> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (
                MemberId::Legacy { raw_id, _phantom },
                MemberId::Legacy {
                    raw_id: other_raw_id,
                    _phantom: _other_phantom,
                },
            ) => raw_id == other_raw_id,
        }
    }
}

impl<Tag> Eq for MemberId<Tag> {}

impl<Tag> Hash for MemberId<Tag> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            MemberId::Legacy { raw_id, _phantom } => raw_id.hash(state),
        }
    }
}
