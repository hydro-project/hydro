use dfir_rs::dfir_syntax;

fn main() {
    let mut df = dfir_syntax! {
        // no generic arguments for `inspect`.
        source_iter(0..10) -> inspect::<usize>(std::mem::drop) -> for_each(std::mem::drop);
    };
    df.run_available_sync();
}
