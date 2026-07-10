fn main() {
    let mut df = dfir_rs::dfir_syntax! {
        source_iter(0..10) -> batch();
        loop {
            batch() -> defer_iteration() -> null();
        };
    };
    df.run_available_sync();
}
