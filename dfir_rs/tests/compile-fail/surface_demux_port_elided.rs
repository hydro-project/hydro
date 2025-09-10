use dfir_rs::{dfir_syntax, var_args};

fn main() {
    let mut df = dfir_syntax! {
        my_demux = source_iter(0..10) -> demux(|item, var_args!(evens, odds)| {
            if 0 == item % 2 {
                evens.give(item);
            }
            else {
                odds.give(item)
            }
        });
        my_demux[evens] -> for_each(std::mem::drop);
        my_demux -> for_each(std::mem::drop);
    };
    df.run_available_sync();
}
