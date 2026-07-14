#![allow(unexpected_cfgs)]

use hydro_lang::prelude::*;

struct P1 {}

fn test<'a>(input: Stream<u32, Process<'a, P1>>) {
    sliced! {
        let s = use(input, nondet!(/** test */));

        // The error inside the initializer body should be attributed to the exact
        // expression that caused it, not the entire `sliced!` block.
        let mut bad = use::state(|l: &Tick<Process<'a, P1>>| {
            let x: u32 = "not a number";
            l.singleton(q!(x))
        });

        bad = bad.clone();
        s
    };
}

fn main() {}
