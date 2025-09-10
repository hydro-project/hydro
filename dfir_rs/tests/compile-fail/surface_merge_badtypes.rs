use dfir_rs::dfir_syntax;

fn main() {
    let mut df = dfir_syntax! {
        m = union() -> for_each(std::mem::drop);
        source_iter(0..10) -> m;
        source_iter("hello".chars()) -> m;
    };
    df.run_available_sync();
}
