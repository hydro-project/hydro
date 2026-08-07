#![allow(unexpected_cfgs)]

use hydro_lang::prelude::*;

struct P1 {}

fn test<'a>(p1: &Process<'a, P1>) {
    // A bounded singleton, materialized only on the first tick.
    let my_count = p1
        .source_iter(q!(0..5i32))
        .fold(q!(|| 0i32), q!(|acc: &mut i32, x| *acc += x));
    let count_mut = my_count.by_mut();

    // An unbounded stream, whose closures run across ticks; a bounded collection is
    // only materialized on the first tick, so accessing the reference on later ticks
    // would crash at runtime. This must not compile.
    let unbounded: Stream<_, _> = p1.source_iter(q!(1..=3i32)).into();
    unbounded.for_each(q!(|x| {
        *count_mut += x;
    }));
}

fn main() {}
