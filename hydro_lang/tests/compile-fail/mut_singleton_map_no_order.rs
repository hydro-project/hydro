#![allow(unexpected_cfgs)]

use hydro_lang::live_collections::stream::NoOrder;
use hydro_lang::prelude::*;

struct P1 {}

fn test<'a>(p1: &Process<'a, P1>) {
    let my_count = p1
        .source_iter(q!(0..5i32))
        .fold(q!(|| 0i32), q!(|acc: &mut i32, x| *acc += x));
    let count_mut = my_count.by_mut();

    let _ = p1
        .source_iter(q!(1..=3i32))
        .weaken_ordering::<NoOrder>()
        .map(q!(|x| {
            *count_mut += x;
            *count_mut
        }));
}

fn main() {}
