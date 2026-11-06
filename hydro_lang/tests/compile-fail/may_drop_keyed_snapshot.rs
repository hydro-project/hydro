#![allow(unexpected_cfgs)]

use hydro_lang::live_collections::stream::MayDrop;
use hydro_lang::prelude::*;

struct P1 {}

// A keyed fold over a `MayDrop` keyed stream is a `MayDrop` keyed singleton, whose keys may never
// reach their final values. Reading it from a tick requires `observe_drops(nonlive!(..))` first.
fn test<'a>(p1: &Process<'a, P1>) {
    let tick = p1.tick();
    let input: Stream<(i32, i32), Process<'a, P1>> =
        p1.source_iter(q!(vec![(1, 2), (1, 3)])).into();
    let _ = input
        .weaken_drops::<MayDrop>()
        .into_keyed()
        .value_counts()
        .snapshot(&tick, nondet!(/** test */));
}

fn main() {}
