use dfir_rs::dfir_syntax;

fn main() {
    let mut df = dfir_syntax! {
        source_iter([(1, 1), (2, 2)]) -> j;
        source_iter([(3, 3), (4, 4)]) -> j;
        j = join() -> for_each(std::mem::drop);
    };
    df.run_available_sync();
}
