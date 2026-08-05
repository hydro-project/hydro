#![allow(unexpected_cfgs)]

use hydro_lang::prelude::*;

struct P1 {}

fn test<'a>(p1: &Process<'a, P1>) {
    // A bounded singleton, materialized only on the first tick.
    let threshold = p1
        .source_iter(q!(0..5i32))
        .fold(q!(|| 0i32), q!(|acc: &mut i32, x| *acc += x));
    let threshold_ref = threshold.by_ref();

    // An unbounded stream, whose closures run across ticks; a bounded collection is
    // only materialized on the first tick, so accessing the reference on later ticks
    // would crash at runtime. This must not compile.
    let unbounded: Stream<_, _> = p1.source_iter(q!(1..=10i32)).into();
    let _ = unbounded.filter(q!(|&x| x > *threshold_ref));
}

fn main() {}
