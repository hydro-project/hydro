use dfir_rs::dfir_syntax;

fn main() {
    let mut df = dfir_syntax! {
        // `inspect` takes no type arguments, so `usize` is extra. A bad lifetime must not hide it.
        source_iter(0..10) -> inspect::<'a, usize>(std::mem::drop) -> for_each(std::mem::drop);
    };
    df.run_available_sync();
}
