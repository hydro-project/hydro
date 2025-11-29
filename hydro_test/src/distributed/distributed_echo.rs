use hydro_lang::live_collections::stream::NoOrder;
use hydro_lang::location::external_process::{ExternalBincodeSink, ExternalBincodeStream};
use hydro_lang::location::{MemberId, MembershipEvent};
use hydro_lang::prelude::*;

pub struct P1 {}
pub struct C2 {}
pub struct C3 {}
pub struct P4 {}
pub struct P5 {}

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
) {
    let (tx, rx) = p1.source_external_bincode(external);

    let rx = rx
        .inspect(q!(|n| println!("received element: {n}")))
        .map(q!(|n| n + 1))
        .inspect(q!(|n| println!("sending element: {n}")))
        .round_robin_bincode(c2, nondet!(/** test */))
        .inspect(q!(|n| println!("received element: {n}")))
        .map(q!(|n| n + 1))
        .inspect(q!(|n| println!("sending element: {n}")))
        .round_robin_bincode(c3, nondet!(/** test */))
        .inspect_with_key(q!(|n| println!("received element: {n:?}")))
        .values()
        .map(q!(|n| n + 1))
        .inspect(q!(|n| println!("sending element: {n}")))
        .send_bincode(p4)
        .inspect_with_key(q!(|n| println!("received element: {n:?}")))
        .values()
        .map(q!(|n| n + 1))
        .inspect(q!(|n| println!("sending element: {n}")))
        .send_bincode(p5)
        .inspect(q!(|n| println!("received element: {n:?}")))
        .map(q!(|n| n + 1))
        .inspect(q!(|n| println!("sending element: {n}")))
        .send_bincode_external(external);

    (tx, rx)
}

#[expect(clippy::type_complexity, reason = "test code")]
pub fn distributed_echo2<'a>(
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
        .inspect(q!(|n| tracing::warn!("received element: {n}")))
        .map(q!(|n| n + 1))
        .inspect(q!(|n| tracing::warn!("sending element: {n}")))
        .round_robin_bincode(c2, nondet!(/** test */))
        .inspect(q!(|n| tracing::warn!("received element: {n}")))
        .map(q!(|n| n + 1))
        .inspect(q!(|n| tracing::warn!("sending element: {n}")))
        .round_robin_bincode(c3, nondet!(/** test */))
        .inspect_with_key(q!(|n| tracing::warn!("received element: {n:?}")))
        .values()
        .map(q!(|n| n + 1))
        .inspect(q!(|n| tracing::warn!("sending element: {n}")))
        .send_bincode(p4)
        .inspect_with_key(q!(|n| tracing::warn!("received element: {n:?}")))
        .values()
        .map(q!(|n| n + 1))
        .inspect(q!(|n| tracing::warn!("sending element: {n}")))
        .send_bincode(p5)
        .inspect(q!(|n| tracing::warn!("received element: {n:?}")))
        .map(q!(|n| n + 1))
        .inspect(q!(|n| tracing::warn!("sending element: {n}")))
        .send_bincode_external(external);

    let ems_c2 = p1
        .source_cluster_members(c2)
        .entries()
        .inspect(q!(|n| tracing::warn!("received membership event: {n:?}")))
        .send_bincode_external(external);

    let ems_c3 = p1
        .source_cluster_members(c3)
        .entries()
        .inspect(q!(|n| tracing::warn!("received membership event: {n:?}")))
        .send_bincode_external(external);

    (tx, rx, ems_c2, ems_c3)
}

// pub struct C2 {}

// pub fn distributed_clustered_echo<'a>(
//     external: &External<'a, ()>,
//     p1: &Process<'a, P1>,
//     c2: &Cluster<'a, C2>,
//     p3: &Process<'a, P3>,
// ) -> (
//     ExternalBincodeSink<String>,
//     ExternalBincodeStream<(MemberId<C2>, String)>,
// ) {
//     let (tx, rx) = p1.source_external_bincode(external);

//     let z = rx
//         .map(q!(|n| {
//             println!("processed element: {n}");
//             format!("{n} - p1")
//         }))
//         .broadcast_bincode(c2, nondet!(/** test */))
//         .map(q!(|n| {
//             println!("processed element: {n}");
//             format!("{n} - c2")
//         }))
//         .send_bincode(p3)
//         .entries()
//         .assume_ordering(nondet!(/** test */))
//         .map(q!(|(from, v)| {
//             println!("processed element: from: {}, v: {v}", from);
//             (from, format!("{v} - p3"))
//         }))
//         .send_bincode_external(external);
//     (tx, z)
// }
