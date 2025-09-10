use dfir_rs::dfir_syntax;

fn main() {
    let mut df = dfir_syntax! {
        src = source_iter(0..10);
        dst = for_each(drop);
        src_0 = [0]src;
        dst_0 = dst[0];
        src_0 -> dst_0;
    };
    df.run_available_sync();
}
