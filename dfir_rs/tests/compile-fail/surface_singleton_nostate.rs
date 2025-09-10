/// Reference an operator that doesn't have singleton state.
pub fn main() {
    let mut df = dfir_rs::dfir_syntax! {
        my_ref = source_iter(15..=25) -> null();
        source_iter(10..=30)
            -> persist::<'static>()
            -> filter(|value| value <= #my_ref.as_reveal_ref())
            -> null();

    };
    df.run_available_sync();
}
