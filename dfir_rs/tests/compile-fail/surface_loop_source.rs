fn main() {
    let mut df = dfir_rs::dfir_syntax! {
        loop {
            source_iter(0..10) -> null();
        };
    };
    df.run_available_sync();
}
