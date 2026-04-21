use dfir_rs::dfir_syntax;

fn main() {
    let mut df = dfir_syntax! {
        f = f;
        f -> for_each(std::mem::drop);
    };
    df.run_available_sync();
}
