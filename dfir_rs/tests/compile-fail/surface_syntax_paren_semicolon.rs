use dfir_rs::dfir_syntax;

fn main() {
    let mut df = dfir_syntax! {
        (source_iter(0..10););
    };
    df.run_available_sync();
}
