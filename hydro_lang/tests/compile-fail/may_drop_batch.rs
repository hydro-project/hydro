#![allow(unexpected_cfgs)]

use hydro_lang::live_collections::stream::MayDrop;
use hydro_lang::prelude::*;

struct P1 {}

// A `MayDrop` stream cannot enter a tick: a batch would expose which elements are missing, and
// the progress a tick relies on has no meaning over a stream whose elements may never arrive.
fn test<'a>(p1: &Process<'a, P1>) {
    let tick = p1.tick();
    let input: Stream<i32, Process<'a, P1>> = p1.source_iter(q!(vec![1, 2, 3])).into();
    let _ = input
        .weaken_drops::<MayDrop>()
        .batch(&tick, nondet!(/** test */));
}

fn main() {}
