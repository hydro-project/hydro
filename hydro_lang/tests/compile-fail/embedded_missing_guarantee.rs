#![allow(unexpected_cfgs)]

use hydro_lang::prelude::*;

struct P1 {}
struct P2 {}

// An `EMBEDDED` channel must declare its retries, ordering, and failure policy.

fn missing_retries<'a>(p1: &Process<'a, P1>, p2: &Process<'a, P2>) {
    let _ = p1
        .source_iter(q!(vec![1, 2, 3]))
        .send(p2, EMBEDDED.total_order().fail_stop().name("no_retries"));
}

fn missing_ordering<'a>(p1: &Process<'a, P1>, p2: &Process<'a, P2>) {
    let _ = p1
        .source_iter(q!(vec![1, 2, 3]))
        .send(p2, EMBEDDED.exactly_once().fail_stop().name("no_ordering"));
}

fn missing_failure_policy<'a>(p1: &Process<'a, P1>, p2: &Process<'a, P2>) {
    let _ = p1.source_iter(q!(vec![1, 2, 3])).send(
        p2,
        EMBEDDED
            .exactly_once()
            .total_order()
            .name("no_failure_policy"),
    );
}

fn main() {}
