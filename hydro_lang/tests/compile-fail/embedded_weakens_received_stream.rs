#![allow(unexpected_cfgs)]

use hydro_lang::live_collections::stream::{ExactlyOnce, TotalOrder};
use hydro_lang::prelude::*;

struct P1 {}
struct P2 {}

fn test<'a>(p1: &Process<'a, P1>, p2: &Process<'a, P2>) {
    let numbers: Stream<_, Process<'a, P1>, Bounded, TotalOrder, ExactlyOnce> =
        p1.source_iter(q!(vec![1, 2, 3]));

    // A `may_duplicate()`, `may_reorder()` channel may duplicate and reorder messages, so even a
    // totally ordered, exactly-once stream is received `AtLeastOnce` and `NoOrder`.
    let _: Stream<_, Process<'a, P2>, Unbounded, TotalOrder, ExactlyOnce> = numbers.send(
        p2,
        EMBEDDED.may_reorder().may_duplicate().never_drops().name("data"),
    );
}

fn main() {}
