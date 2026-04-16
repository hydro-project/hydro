use dfir_rs::dfir_syntax;

fn main() {
    let mut df = dfir_syntax! {
        f = f -> map(|x: usize| x) -> f;
    };
    df.run_available_sync();
}
