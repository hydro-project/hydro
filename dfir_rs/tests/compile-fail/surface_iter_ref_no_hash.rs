/// iter_ref() requires a `#handoff_name` reference as its argument.
fn main() {
    let my_vec = vec![1_i32, 2, 3];
    let mut df = dfir_rs::dfir_syntax! {
        iter_ref(my_vec) -> for_each(|v: &i32| println!("{v}"));
    };
    df.run_available_sync();
}
