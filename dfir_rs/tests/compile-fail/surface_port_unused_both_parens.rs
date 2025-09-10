use dfir_rs::dfir_syntax;

fn main() {
    let mut df = dfir_syntax! {
        src = [0](source_iter(0..10));
        dst = (for_each(drop))[0];
        src -> dst;
    };
    df.run_available_sync();
}
