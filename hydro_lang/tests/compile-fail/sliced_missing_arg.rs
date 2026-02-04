use hydro_lang::prelude::*;

struct P1 {}

fn test<'a>(input: Stream<u32, Process<'a, P1>>) {
    sliced! {
        let a = use(input);
        let b = use::atomic(input.atomic(&input.location().tick()));
        let c = use(input, 123);
        let d = use::atomic(input.atomic(&input.location().tick()), 123);
        let e = use(fake, nondet!(/** test */));
        let f = use::atomic(fake, nondet!(/** test */));
        let g = use::foobar();
    }
}

fn main() {}
