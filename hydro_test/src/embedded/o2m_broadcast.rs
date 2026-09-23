use hydro_lang::prelude::*;

pub struct Src {}
pub struct Dst {}

pub fn o2m_broadcast<'a>(
    cluster: &Cluster<'a, Dst>,
    input: Stream<String, Process<'a, Src>>,
) -> Stream<String, Cluster<'a, Dst>> {
    input
        .broadcast(
            cluster,
            TCP.fail_stop().bincode().name("o2m_data"),
            nondet!(/** test */),
        )
        .map(q!(|s| s.to_uppercase()))
}
