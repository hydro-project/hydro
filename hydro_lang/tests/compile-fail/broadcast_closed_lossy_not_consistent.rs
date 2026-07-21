#![allow(unexpected_cfgs)]

use hydro_lang::location::cluster::EventualConsistency;
use hydro_lang::prelude::*;

struct P1 {}
struct Workers {}

fn test<'a>(p1: &Process<'a, P1>, workers: &Cluster<'a, Workers>) {
    let numbers = p1.source_iter(q!(vec![123]));

    // A `lossy` network can drop messages for some members while delivering them to
    // others, so the output of `broadcast_closed` is only `NoConsistency`.
    let _: Stream<_, Cluster<'a, Workers, EventualConsistency>, _, _, _> =
        numbers.broadcast_closed(workers, TCP.lossy(nondet!(/** test */)).bincode());
}

fn main() {}
