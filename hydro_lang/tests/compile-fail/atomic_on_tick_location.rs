#![allow(unexpected_cfgs)]

use hydro_lang::prelude::*;

struct P1 {}

fn test<'a>(p1: &Process<'a, P1>) {
    let tick = p1.tick();
    let batched = p1
        .source_iter(q!(0..10))
        .batch(&tick, nondet!(/** test */));

    // `.atomic()` on a tick-located stream would create an invalid nested
    // `Tick<Tick<Process>>` location.
    let _ = batched.atomic();
}

fn main() {}
