use dfir_rs::{dfir_syntax, var_args};

fn main() {
    let mut df = dfir_syntax! {
        my_demux = source_iter(0..10) -> demux(|item, var_args!(a, b, c)| {
            match item % 3 {
                0 => a.give(item),
                1 => b.give(item),
                2 => c.give(item),
            }
        });
        my_demux[b] -> for_each(std::mem::drop);
        my_demux[c] -> for_each(std::mem::drop);
        my_demux[d] -> for_each(std::mem::drop);
    };
    df.run_available_sync();
}
