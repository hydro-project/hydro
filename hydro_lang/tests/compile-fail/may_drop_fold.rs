#![allow(unexpected_cfgs)]

use hydro_lang::live_collections::stream::MayDrop;
use hydro_lang::prelude::*;

struct P1 {}

// An ordered `MayDrop` stream may be missing elements anywhere. A non-commutative fold has a
// single trajectory, and its value after a missing element lies on none of it, so the closure
// must be proven commutative (or the drops observed first).
fn test<'a>(p1: &Process<'a, P1>) {
    let input: Stream<char, Process<'a, P1>> = p1.source_iter(q!(vec!['a', 'b', 'c'])).into();
    let _ = input
        .weaken_drops::<MayDrop>()
        .fold(q!(|| String::new()), q!(|acc, c| acc.push(c)));
}

fn main() {}
