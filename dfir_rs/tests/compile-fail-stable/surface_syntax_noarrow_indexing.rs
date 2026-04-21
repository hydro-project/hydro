use dfir_rs::dfir_syntax;

fn main() {
    let mut df = dfir_syntax! {
        x = for_each(std::mem::drop);
        source_iter(0..10) [0]x;
    };
    df.run_available_sync();
}
