use hydro_lang::live_collections::stream::NoOrder;
use hydro_lang::location::external_process::{ExternalBincodeSink, ExternalBincodeStream};
use hydro_lang::prelude::*;

pub struct P1 {}
pub struct P2 {}
pub struct C3 {}
pub struct C4 {}
pub struct P5 {}

pub fn distributed_echo<'a>(
    external: &External<'a, ()>,
    p1: &Process<'a, P1>,
    c2: &Cluster<'a, C3>,
    c3: &Cluster<'a, C4>,
    p4: &Process<'a, P5>,
    p5: &Process<'a, P5>,
) -> (
    ExternalBincodeSink<u32>,
    ExternalBincodeStream<u32, NoOrder>,
) {
    let (tx, rx) = p1.source_external_bincode(external);

    let z = rx
        .map(q!(|n| n + 1))
        .round_robin_bincode(c2, nondet!(/** test */))
        .map(q!(|n| n + 1))
        .round_robin_bincode(c3, nondet!(/** test */))
        .values()
        .map(q!(|n| n + 1))
        .send_bincode(p4)
        .values()
        .map(q!(|n| n + 1))
        .send_bincode(p5)
        .map(q!(|n| n + 1))
        .send_bincode_external(external);

    (tx, z)
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
