#![allow(unexpected_cfgs)]

use hydro_lang::live_collections::stream::MayDrop;
use hydro_lang::prelude::*;

struct P1 {}

// A fold over a `MayDrop` stream is a `MayDrop` singleton, which may never reach the end of its
// trajectory. Reading it from a tick requires `observe_drops(nonlive!(..))` first.
fn test<'a>(p1: &Process<'a, P1>) {
    let tick = p1.tick();
    let input: Stream<i32, Process<'a, P1>> = p1.source_iter(q!(vec![1, 2, 3])).into();
    let _ = input
        .weaken_drops::<MayDrop>()
        .count()
        .snapshot(&tick, nondet!(/** test */));
}

fn main() {}
