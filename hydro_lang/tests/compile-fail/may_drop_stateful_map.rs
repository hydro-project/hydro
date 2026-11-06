#![allow(unexpected_cfgs)]

use hydro_lang::live_collections::stream::MayDrop;
use hydro_lang::prelude::*;

struct P1 {}

// A closure that mutably captures state cannot observe a `MayDrop` stream: its state would
// depend on which elements are missing. Only stateless (`Fn`) closures are accepted, so the
// assignment to the captured state does not compile.
fn test<'a>(p1: &Process<'a, P1>) {
    let input: Stream<i32, Process<'a, P1>> = p1.source_iter(q!(1..=3i32)).into();
    let _ = input.weaken_drops::<MayDrop>().map(q!({
        let mut total = 0;
        move |x| {
            total += x;
            total
        }
    }));
}

fn main() {}
