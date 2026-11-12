use std::fmt::Debug;
use std::hash::Hash;

use hydro_lang::location::cluster::CLUSTER_SELF_ID;
use hydro_lang::prelude::*;

pub struct Src {}
pub struct Dst {}

/// Like [`super::o2m_broadcast::o2m_broadcast`], but generic over `Id`, the member ID type.
/// That lets an embedded host name members however it likes.
/// Each member tags what it receives with its own ID.
pub fn broadcast_to_members<'a, Id: Clone + Eq + Hash + Debug + 'a>(
    cluster: &Cluster<'a, Dst, Id>,
    input: Stream<String, Process<'a, Src>>,
) -> Stream<String, Cluster<'a, Dst, Id>> {
    input
        .broadcast(
            cluster,
            EMBEDDED
                .never_reorders()
                .never_duplicates()
                .never_drops()
                .name("to_members"),
            nondet!(/** test */),
        )
        .map(q!(move |s| format!("{:?} got {s}", CLUSTER_SELF_ID)))
}
