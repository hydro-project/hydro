use hydro_lang::prelude::*;

struct P1 {}

fn test<'a>(p1: &Process<'a, P1>) {
    p1.source_iter(q!(0..10)).weakest_ordering().fold(
        q!(|| 0),
        q!(|acc, x| *acc += x),
    );
}

fn main() {}
