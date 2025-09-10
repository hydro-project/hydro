fn main() {
    let mut df = dfir_rs::dfir_syntax! {
        a = source_iter(0..10);
        loop {
            b = a -> batch();
        };
        b -> null();
    };
    df.run_available_sync();
}
