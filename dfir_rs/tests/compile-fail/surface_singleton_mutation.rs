/// Consumers should not be able to mutate singleton state.
fn main() {
    let mut df = dfir_rs::dfir_syntax! {
        stream2 = source_iter(3..=5);
        max_of_stream2 = stream2 -> fold(|| 0usize, |a, b| *a = std::cmp::max(*a, b));

        source_iter(1..=3)
            -> map(|x| {
                #max_of_stream2 = 999;
                x
            })
            -> for_each(|x| println!("{}", x));
    };
    df.run_available_sync();
}
