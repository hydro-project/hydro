use dfir_rs::dfir_syntax;

fn main() {
    let mut df = dfir_syntax! {
        my_partition = source_iter(0..10) -> partition(std::mem::drop);
        my_partition[a] -> for_each(std::mem::drop);
        my_partition[b] -> for_each(std::mem::drop);
        my_partition[c] -> for_each(std::mem::drop);
    };
    df.run_available_sync();
}
