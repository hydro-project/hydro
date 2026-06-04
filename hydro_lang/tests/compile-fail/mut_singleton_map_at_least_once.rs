use hydro_lang::live_collections::stream::AtLeastOnce;
use hydro_lang::prelude::*;

struct P1 {}

fn test<'a>(p1: &Process<'a, P1>) {
    let my_count = p1
        .source_iter(q!(0..5i32))
        .fold(q!(|| 0i32), q!(|acc: &mut i32, x| *acc += x));
    let count_mut = my_count.by_mut();

    let _ = p1
        .source_iter(q!(1..=3i32))
        .weaken_retries::<AtLeastOnce>()
        .map(q!(|x| {
            *count_mut += x;
            *count_mut
        }));
}

fn main() {}
