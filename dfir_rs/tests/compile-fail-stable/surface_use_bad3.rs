use dfir_rs::dfir_syntax;

fn main() {
    let mut df = dfir_syntax! {
        use;

        source_iter(0..10) -> for_each(std::mem::drop);
    };
    df.run_available_sync();
}
