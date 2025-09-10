use dfir_rs::dfir_syntax;

fn main() {
    let mut df = dfir_syntax! {
        j = anti_join() -> for_each(std::mem::drop);
        source_iter(0..10) -> map(|x| (x, x)) -> [pos]j;
        source_iter(0..10) -> map(|_| "string") -> [neg]j;
    };
    df.run_available_sync();
}
