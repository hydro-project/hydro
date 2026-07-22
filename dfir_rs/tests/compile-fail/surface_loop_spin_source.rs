fn main() {
    let mut df = dfir_rs::dfir_syntax! {
        loop {
            spin() -> null();
        };
    };
    df.run_available_sync();
}
