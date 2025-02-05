use hydro_lang::*;

pub fn first_ten(process: &Process) {
    process
        .source_iter(q!(0..10))
        .for_each(q!(|n| println!("{}", n)));
}
