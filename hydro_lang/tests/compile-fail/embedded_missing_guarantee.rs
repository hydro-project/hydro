#![allow(unexpected_cfgs)]

use hydro_lang::prelude::*;

struct P1 {}
struct P2 {}

// An `EMBEDDED` channel must declare, for each anomaly, whether it may introduce it.

fn missing_duplication<'a>(p1: &Process<'a, P1>, p2: &Process<'a, P2>) {
    let _ = p1
        .source_iter(q!(vec![1, 2, 3]))
        .send(p2, EMBEDDED.never_reorders().never_drops().name("no_dups"));
}

fn missing_reordering<'a>(p1: &Process<'a, P1>, p2: &Process<'a, P2>) {
    let _ = p1.source_iter(q!(vec![1, 2, 3])).send(
        p2,
        EMBEDDED.never_duplicates().never_drops().name("no_order"),
    );
}

fn missing_loss<'a>(p1: &Process<'a, P1>, p2: &Process<'a, P2>) {
    let _ = p1.source_iter(q!(vec![1, 2, 3])).send(
        p2,
        EMBEDDED.never_reorders().never_duplicates().name("no_loss"),
    );
}

fn main() {}
