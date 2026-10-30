#![allow(unexpected_cfgs)]

use hydro_lang::prelude::*;

struct P1 {}

fn test<'a>(p1: &Process<'a, P1>) {
    let input = p1.source_iter(q!(1..=3i32));

    // An unbounded singleton, whose value keeps changing over time.
    let unbounded: Stream<_, _> = p1.source_iter(q!(0..5i32)).into();
    let count = unbounded.fold(q!(|| 0i32), q!(|acc: &mut i32, x| *acc += x));

    // Reading an unbounded singleton in a slice is non-deterministic, so it requires
    // `use::snapshot` with a `nondet!` guard. `use::clone` must not compile.
    let _ = sliced! {
        let batch = use::batch(input, nondet!(/** test */));
        let count = use::clone(count);
        batch.cross_singleton(count)
    };
}

fn main() {}
