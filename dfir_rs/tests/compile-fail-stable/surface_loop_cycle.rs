fn main() {
    let mut df = dfir_rs::dfir_syntax! {
        loop {
            a = identity() -> identity() -> identity() -> identity();
            a -> a;
        };
    };
    df.run_available_sync();
}
