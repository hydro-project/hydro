use hydro_lang::live_collections::stream::NoOrder;
use hydro_lang::prelude::*;

struct P1 {}

fn test<'a>(p1: &Process<'a, P1>) {
    let _ = p1.source_iter(q!(0..10)).weaken_ordering::<NoOrder>().enumerate();
}

fn main() {}
