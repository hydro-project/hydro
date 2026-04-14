fn main() {
    let mut flow = dfir_rs::dfir_syntax_inline! {
        source_iter(0..5_i32) -> defer_tick() -> for_each(|_x: i32| {});
    };
}
