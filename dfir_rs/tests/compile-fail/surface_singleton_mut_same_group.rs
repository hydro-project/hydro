/// Multiple `#mut` references in the same access group should be an error.
fn main() {
    let mut df = dfir_rs::dfir_syntax! {
        my_val = source_iter([0_i32]) -> singleton();
        my_val -> for_each(|_| {});
        source_iter([1_i32]) -> map(|x| { *#{0} mut my_val += x; x }) -> for_each(|_| {});
        source_iter([2_i32]) -> map(|x| { *#{0} mut my_val += x; x }) -> for_each(|_| {});
    };
    df.run_available_sync();
}
