use hydro_lang::prelude::*;

pub fn first_ten<'a>(process: &Process<'a, ()>) {
    process
        .source_iter(q!(0..10))
        .for_each(q!(|n| println!("{}", n)));
}
