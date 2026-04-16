use dfir_rs::dfir_syntax;

fn main() {
    let mut df = dfir_syntax! {
        a = null() -> null();
        a = null() -> null();
    };
    df.run_available_sync();
}
