#![allow(unexpected_cfgs)]

use hydro_lang::live_collections::stream::MayDrop;
use hydro_lang::prelude::*;

struct P1 {}

// The values of each key of a `MayDrop` keyed stream may be missing anywhere, so a fold over them
// must be proven commutative (or the drops observed first), even when the values are ordered.
fn test<'a>(p1: &Process<'a, P1>) {
    let input: Stream<(i32, i32), Process<'a, P1>> =
        p1.source_iter(q!(vec![(1, 2), (1, 3)])).into();
    let _ = input
        .weaken_drops::<MayDrop>()
        .into_keyed()
        .fold(q!(|| 0), q!(|acc, x| *acc += x));
}

fn main() {}
