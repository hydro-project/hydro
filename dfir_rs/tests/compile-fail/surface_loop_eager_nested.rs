fn main() {
    let mut df = dfir_rs::dfir_syntax! {
        a = source_iter(0..10);
        loop {
            a -> batch() -> b;
            b = identity();
            loop {
                b -> batch_eager() -> null();
            };
        };
    };
    df.run_available_sync();
}
