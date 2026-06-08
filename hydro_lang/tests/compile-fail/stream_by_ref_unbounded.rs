use hydro_lang::prelude::*;

struct P1 {}

fn test<'a>(p1: &Process<'a, P1>) {
    let unbounded: Stream<_, _> = p1.source_iter(q!(0..10)).into();
    let _ = unbounded.by_ref();
}

fn main() {}
