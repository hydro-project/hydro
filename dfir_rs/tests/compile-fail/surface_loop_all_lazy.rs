fn main() {
    let mut df = dfir_rs::dfir_syntax! {
        a = source_iter(0..10);
        b = source_iter(0..10);
        loop {
            m = union();
            a -> snapshot() -> m;
            b -> batch_lazy() -> m;
            m -> null();
        };
    };
    df.run_available_sync();
}
