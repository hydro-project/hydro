use hydro_lang::location::MemberId;
use hydro_lang::location::cluster::CLUSTER_SELF_ID;
use hydro_lang::prelude::*;

pub struct Members {}

/// Tags each element with the ID of the cluster member that receives it, so the generated
/// function takes the member's own ID.
pub fn tag_with_self_id<'a>(
    input: Stream<String, Cluster<'a, Members>>,
) -> Stream<(MemberId<Members>, String), Cluster<'a, Members>> {
    input.map(q!(move |s| (CLUSTER_SELF_ID.clone(), s)))
}
