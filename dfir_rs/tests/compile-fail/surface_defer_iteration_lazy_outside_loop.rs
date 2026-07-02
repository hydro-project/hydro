fn main() {
    let mut df = dfir_rs::dfir_syntax! {
        source_iter(0..10) -> defer_iteration_lazy() -> null();
    };
    df.run_available_sync();
}
