use dfir_rs::dfir_syntax;

/// Technically this should/could compile, but it is a completely useless edge case.
fn main() {
    let mut df = dfir_syntax! {
        null();
    };
    df.run_available_sync();
}
