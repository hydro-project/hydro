use dfir_rs::dfir_syntax;

fn main() {
    let mut df = dfir_syntax! {
        my_partition = source_iter(0..10) -> partition(|item, [foo, 0]| {
            if item % 2 == 0 { foo } else { 0 }
        });
        my_partition[foo] -> for_each(std::mem::drop);
        my_partition[bar] -> for_each(std::mem::drop);
    };
    df.run_available_sync();
}
