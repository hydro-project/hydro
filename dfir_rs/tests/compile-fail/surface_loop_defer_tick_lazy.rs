fn main() {
    let mut df = dfir_rs::dfir_syntax! {
        inp = source_iter(0..10);
        loop {
            inp -> batch() -> inner_data;
            inner_data = identity();
            loop {
                inner_data -> batch() -> defer_tick_lazy() -> null();
            };
        };
    };
    df.run_available_sync();
}
