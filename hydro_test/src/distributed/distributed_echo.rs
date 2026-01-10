use hydro_lang::live_collections::stream::NoOrder;
use hydro_lang::location::external_process::{ExternalBincodeSink, ExternalBincodeStream};
use hydro_lang::location::{Location, MemberId, MembershipEvent};
use hydro_lang::prelude::*;

pub struct P1 {}
pub struct C2 {}
pub struct C3 {}
pub struct P4 {}
pub struct P5 {}

#[expect(clippy::type_complexity, reason = "test code")]
pub fn distributed_echo<'a>(
    external: &External<'a, ()>,
    p1: &Process<'a, P1>,
    c2: &Cluster<'a, C2>,
    c3: &Cluster<'a, C3>,
    p4: &Process<'a, P4>,
    p5: &Process<'a, P5>,
) -> (
    ExternalBincodeSink<u32>,
    ExternalBincodeStream<u32, NoOrder>,
    ExternalBincodeStream<(MemberId<C2>, MembershipEvent), NoOrder>,
    ExternalBincodeStream<(MemberId<C3>, MembershipEvent), NoOrder>,
) {
    let (tx, rx) = p1.source_external_bincode(external);

    let rx = rx
        .map(q!(|n| n + 1))
        .round_robin(c2, TCP.bincode(), nondet!(/** test */))
        .map(q!(|n| n + 1))
        .round_robin(c3, TCP.bincode(), nondet!(/** test */))
        .values()
        .map(q!(|n| n + 1))
        .send(p4, TCP.bincode())
        .values()
        .map(q!(|n| n + 1))
        .send(p5, TCP.bincode())
        .map(q!(|n| n + 1))
        .send_bincode_external(external);

    let ems_c2 = p1
        .source_cluster_members(c2)
        .entries()
        .send_bincode_external(external);

    let ems_c3 = p1
        .source_cluster_members(c3)
        .entries()
        .send_bincode_external(external);

    (tx, rx, ems_c2, ems_c3)
}
