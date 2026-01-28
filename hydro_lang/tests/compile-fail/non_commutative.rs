use hydro_lang::live_collections::stream::NoOrder;
use hydro_lang::prelude::*;

struct P1 {}

fn test<'a>(p1: &Process<'a, P1>) {
    p1.source_iter(q!(0..10))
        .weaken_ordering::<NoOrder>()
        .fold(q!(|| 0), q!(|acc, x| *acc += x));
}

fn main() {}
