/// Correct reference but using it wrong (bad expression).
fn main() {
    let mut df = dfir_rs::dfir_syntax! {
        my_ref = source_iter(15_u32..=25) -> fold(|| 0, |a, b| *a = std::cmp::max(*a, b));
        source_iter(10_u32..=30)
            -> persist()
            -> filter(|value| value <= #my_ref)
            -> null();

    };
    df.run_available_sync();
}
