use dfir_rs::dfir_syntax;

fn main() {
    let mut df = dfir_syntax! {
        my_partition = source_iter(0..10) -> partition(|item, [evens, _0]| {
            if 0 == item % 2 {
                evens
            }
            else {
                _0
            }
        });
        my_partition[evens] -> for_each(std::mem::drop);
        my_partition[0] -> for_each(std::mem::drop);
    };
    df.run_available_sync();
}
