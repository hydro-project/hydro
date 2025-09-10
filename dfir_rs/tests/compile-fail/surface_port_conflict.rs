use dfir_rs::dfir_syntax;

fn main() {
    let mut df = dfir_syntax! {
        t = source_iter([1]) -> tee();
        t[1] -> for_each(std::mem::drop);
        t[1] -> for_each(std::mem::drop);
    };
    df.run_available_sync();
}
