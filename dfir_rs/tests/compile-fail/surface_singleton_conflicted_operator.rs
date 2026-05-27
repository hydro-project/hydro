/// Multiple access groups in the same operator should be an error.
fn main() {
    let mut df = dfir_rs::dfir_syntax! {
        my_val = source_iter([0_i32]) -> singleton();
        my_val -> for_each(|_| {});
        source_iter(0..10) -> fold(|| #{0} my_val, |a, b| *a = b + #{1} my_val) -> for_each(|_| {});
    };
    df.run_available_sync();
}
