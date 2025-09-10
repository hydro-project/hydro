use dfir_rs::dfir_syntax;

fn main() {
    let mut df = dfir_syntax! {
        // Due to the TODO the `usize` is not caught, but that is ok.
        source_iter(0..10) -> identity::<'a, usize>() -> for_each(std::mem::drop);
    };
    df.run_available_sync();
}
