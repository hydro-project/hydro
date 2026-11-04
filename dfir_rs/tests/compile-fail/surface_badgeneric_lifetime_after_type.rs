use dfir_rs::dfir_syntax;

fn main() {
    let mut df = dfir_syntax! {
        // A lifetime after a type is not persistence, including `'tick`.
        source_iter(0..10) -> identity::<usize, 'a>() -> for_each(std::mem::drop);
        source_iter(0..10) -> identity::<usize, 'tick>() -> for_each(std::mem::drop);
    };
    df.run_available_sync();
}
