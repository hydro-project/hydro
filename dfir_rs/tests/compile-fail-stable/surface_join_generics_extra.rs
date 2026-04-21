use dfir_rs::dfir_syntax;

fn main() {
    let mut df = dfir_syntax! {
        j = join::<usize, usize>() -> for_each(std::mem::drop);
        source_iter(0..10) -> map(|x| (x, x)) -> [0]j;
        source_iter(0..10) -> map(|x| (x, x)) -> [1]j;
    };
    df.run_available_sync();
}
