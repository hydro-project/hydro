fn main() {
    let mut flow = dfir_rs::dfir_syntax_inline! {
        source_iter(0..5_i32) -> my_union;
        my_union = union() -> map(|x: i32| x + 1) -> my_union;
    };
}
