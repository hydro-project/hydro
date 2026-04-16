use dfir_rs::dfir_syntax;

fn main() {
    let mut df = dfir_syntax! {
        src = source_iter(0..10);
        [0]src -> for_each(drop);
    };
    df.run_available_sync();
}
