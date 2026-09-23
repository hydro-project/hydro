use hydro_lang::live_collections::stream::NoOrder;
use hydro_lang::location::MemberId;
use hydro_lang::prelude::*;

pub struct Src {}
pub struct Dst {}

pub fn m2o_send<'a>(
    process: &Process<'a, Dst>,
    input: Stream<String, Cluster<'a, Src>>,
) -> Stream<(MemberId<Src>, String), Process<'a, Dst>, Unbounded, NoOrder> {
    input
        .send(process, TCP.fail_stop().bincode().name("m2o_data"))
        .map(q!(|s| s.to_uppercase()))
        .entries()
}
