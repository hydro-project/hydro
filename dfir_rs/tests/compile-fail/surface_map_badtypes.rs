use dfir_rs::dfir_syntax;

fn main() {
    let mut df = dfir_syntax! {
        source_iter(0..10) -> map(|(a, b)| a + b) -> for_each(std::mem::drop);
    };
    df.run_available_sync();
}
