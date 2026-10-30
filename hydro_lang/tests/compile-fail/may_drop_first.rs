#![allow(unexpected_cfgs)]

use hydro_lang::live_collections::stream::MayDrop;
use hydro_lang::prelude::*;

struct P1 {}

// The first element of an ordered `MayDrop` stream to arrive need not head any prefix of the
// stream, so `first` is not available (unlike `last`).
fn test<'a>(p1: &Process<'a, P1>) {
    let input: Stream<i32, Process<'a, P1>> = p1.source_iter(q!(vec![1, 2, 3])).into();
    let _ = input.weaken_drops::<MayDrop>().first();
}

fn main() {}
