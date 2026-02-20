use hydro_lang::live_collections::stream::NoOrder;
use hydro_lang::location::MemberId;
use hydro_lang::prelude::*;

pub struct Src {}
pub struct Dst {}

pub fn m2m_broadcast<'a>(
    dst: &Cluster<'a, Dst>,
    input: Stream<String, Cluster<'a, Src>>,
) -> Stream<(MemberId<Src>, String), Cluster<'a, Dst>, Unbounded, NoOrder> {
    // used to test that we can call `source_cluster members` multiple times
    input
        .location()
        .source_cluster_members(dst)
        .entries()
        .assume_ordering(nondet!(/** test */))
        .for_each(q!(|_| {}));

    input
        .broadcast(
            dst,
            TCP.fail_stop().bincode().name("m2m_data"),
            nondet!(/** test */),
        )
        .map(q!(|s| s.to_uppercase()))
        .entries()
}
