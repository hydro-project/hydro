#![allow(unexpected_cfgs)]

use std::marker::PhantomData;

use hydro_lang::prelude::*;

struct P1 {}

struct Clock<'a, Initializer, Tag>
where
    Tag: 'a,
    // The signature here should be `Fn(&Tick<...>)` (generic over the reference lifetime),
    // not `Fn(&'a Tick<...>)`. This mistake causes the initializer to only implement `Fn`
    // for one specific lifetime, which is incompatible with `use::state`.
    Initializer: Fn(&'a Tick<Process<'a, Tag>>) -> Singleton<u32, Tick<Process<'a, Tag>>, Bounded>,
{
    initializer: Initializer,
    _phantom: PhantomData<&'a Tag>,
}

fn test<'a>(input: Stream<u32, Process<'a, P1>>) {
    let clock = Clock {
        initializer: |l: &Tick<Process<'a, P1>>| l.singleton(q!(0)),
        _phantom: PhantomData,
    };

    sliced! {
        let s = use(input, nondet!(/** test */));

        let mut bad = use::state(&clock.initializer);

        s
    };
}

fn main() {}
