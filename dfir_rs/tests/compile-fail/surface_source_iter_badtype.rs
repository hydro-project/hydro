use dfir_rs::dfir_syntax;

fn main() {
    let mut df = dfir_syntax! {
        source_iter(()) -> for_each(std::mem::drop);
    };
    df.run_available();
}
