#![allow(unexpected_cfgs)]

use hydro_lang::prelude::*;

struct P1 {}
struct P2 {}

// An `EMBEDDED` channel either may drop messages or may delay them forever, not both.
fn test<'a>(p1: &Process<'a, P1>, p2: &Process<'a, P2>) {
    let _ = p1.source_iter(q!(vec![1, 2, 3])).send(
        p2,
        EMBEDDED
            .never_reorders()
            .never_duplicates()
            .may_drop(nondet!(/** test */))
            .may_delay_forever()
            .name("data"),
    );
}

fn main() {}
