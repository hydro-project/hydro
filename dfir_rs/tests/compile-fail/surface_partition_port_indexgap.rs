use dfir_rs::dfir_syntax;

fn main() {
    let mut df = dfir_syntax! {
        my_partition = source_iter(0..10) -> partition(|item, n| item % n);
        my_partition[0] -> for_each(std::mem::drop);
        my_partition[1] -> for_each(std::mem::drop);
        my_partition[3] -> for_each(std::mem::drop);
    };
    df.run_available_sync();
}
