use dfir_rs::dfir_syntax;

fn main() {
    let mut df = dfir_syntax! {
        src = source_iter(0..10);
        dst = for_each(drop);
        [0]src -> dst[0];
    };
    df.run_available_sync();
}
