fn main() {
    let mut df = dfir_rs::dfir_syntax! {
        a = source_iter(0..10);
        loop {
            loop {
                a -> batch() -> null();
            };
        };
    };
    df.run_available_sync();
}
