/// Ungrouped shared refs cannot coexist with grouped mutable refs to the same singleton.
fn main() {
    let mut df = dfir_rs::dfir_syntax! {
        my_val = source_iter([0_i32]) -> singleton();
        my_val -> for_each(|_| {});
        source_iter([1_i32]) -> map(|x| x + #my_val) -> for_each(|_| {});
        source_iter([2_i32]) -> map(|x| { *#{0} mut my_val += x; x }) -> for_each(|_| {});
    };
    df.run_available_sync();
}
