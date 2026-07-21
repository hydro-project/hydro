fn main() {
    let mut df = dfir_rs::dfir_syntax! {
        my_buf = source_iter(0..10) -> handoff();
        my_buf -> null();
        loop {
            iter_ref(#my_buf) -> null();
        };
    };
    df.run_available_sync();
}
