use dfir_rs::dfir_syntax;

fn main() {
    let mut df = dfir_syntax! {
        my_demux = source_iter(0..10) -> demux(std::mem::drop);
        my_demux[a] -> for_each(std::mem::drop);
        my_demux[b] -> for_each(std::mem::drop);
        my_demux[c] -> for_each(std::mem::drop);
    };
    df.run_available_sync();
}
