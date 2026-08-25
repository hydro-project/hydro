#![allow(unexpected_cfgs)]

use hydro_lang::prelude::*;

struct P1 {}

fn test<'a>(p1: &Process<'a, P1>) {
    // `.atomic()` on an already-atomic stream would create a redundant
    // `Atomic<Atomic<Process>>` location.
    let _ = p1.source_iter(q!(0..10)).atomic().atomic();
}

fn main() {}
